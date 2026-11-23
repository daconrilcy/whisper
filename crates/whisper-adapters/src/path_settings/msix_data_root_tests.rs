use super::validate_msix_local_data_root;
use std::path::PathBuf;

#[test]
fn absent_package_identity_does_not_create_a_msix_root() {
    assert_eq!(validate_msix_local_data_root(false, None), Ok(None));
}

#[test]
fn package_identity_requires_an_absolute_local_folder() {
    let root = PathBuf::from(r"C:\Users\tester\AppData\Local\Packages\Whisper\LocalState");
    assert_eq!(
        validate_msix_local_data_root(true, Some(root.clone())),
        Ok(Some(root))
    );
    assert_eq!(
        validate_msix_local_data_root(true, Some(PathBuf::from("relative"))),
        Err("MSIX LocalFolder path is not absolute".into())
    );
    assert_eq!(
        validate_msix_local_data_root(true, None),
        Err("MSIX LocalFolder missing".into())
    );
}
