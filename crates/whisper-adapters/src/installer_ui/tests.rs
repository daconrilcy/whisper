use super::{OPENGL_COMPATIBILITY_PACK_STORE_URI, is_missing_glow_compatibility_error};

#[test]
fn recognizes_the_reported_glow_opengl_20_failure() {
    let diagnostic = "AppCreation(Custom { error: \"egui_glow requires opengl 2.0+.\" })";

    assert!(is_missing_glow_compatibility_error(diagnostic));
}

#[test]
fn ignores_other_eframe_startup_errors() {
    assert!(!is_missing_glow_compatibility_error(
        "AppCreation(Custom { error: \"window creation failed\" })"
    ));
    assert!(!is_missing_glow_compatibility_error(
        "OpenGL driver failed for an unrelated reason"
    ));
}

#[test]
fn store_uri_targets_the_verified_compatibility_pack_product() {
    assert_eq!(
        OPENGL_COMPATIBILITY_PACK_STORE_URI,
        "ms-windows-store://pdp/?ProductId=9NQPSL29BFFF"
    );
}
