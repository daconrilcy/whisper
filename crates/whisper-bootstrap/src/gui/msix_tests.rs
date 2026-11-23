use std::path::PathBuf;

use super::{StartupAction, packaged_desktop_path, startup_action};

#[test]
fn packaged_identity_routes_to_desktop_without_install_action() {
    let package_local_data =
        PathBuf::from(r"C:\Users\test\AppData\Local\Packages\Whisper\LocalState");
    let action = startup_action(Ok(Some(package_local_data.clone())), None)
        .expect("package identity should select the packaged desktop");

    assert_eq!(
        action,
        StartupAction::LaunchPackaged {
            local_data: package_local_data.clone(),
        }
    );
    assert_eq!(action.local_app_data(), Some(package_local_data.as_path()));
}

#[test]
fn missing_package_identity_preserves_the_zip_install_route_and_path() {
    let local_app_data = PathBuf::from(r"C:\Users\test\AppData\Local");
    let action = startup_action(Ok(None), Some(local_app_data.clone()))
        .expect("unpackaged process should retain the ZIP installer route");

    assert_eq!(
        action,
        StartupAction::Install {
            local_app_data: Some(local_app_data),
        }
    );
}

#[test]
fn package_identity_lookup_error_fails_closed() {
    let action = startup_action(Err("MSIX identity lookup failed".into()), None);

    assert_eq!(action, Err("MSIX identity lookup failed".into()));
}

#[test]
fn packaged_route_targets_the_desktop_inside_the_package_root() {
    let package_root = PathBuf::from(r"C:\package");

    assert_eq!(
        packaged_desktop_path(&package_root),
        package_root.join("app").join("whisper-desktop.exe")
    );
}
