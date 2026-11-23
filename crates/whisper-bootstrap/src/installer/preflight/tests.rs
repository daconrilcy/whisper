use super::{DestinationRequirement, preflight_bundle_with_assets};
use crate::installer::{
    BundleAsset, InstallError, InstallRequest, copy_new_file, digest_hex, install_with,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use whisper_adapters::archive::sha256_file;
use whisper_core::cpu_compatibility::{CpuCompatibility, CpuCompatibilityLevel};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestArea(PathBuf);

impl TestArea {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "whisper-bootstrap-preflight-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("isolated test directory is created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestArea {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Fixture {
    local_app_data: PathBuf,
    bundle_root: PathBuf,
    model_path: PathBuf,
    bundle_assets: Vec<BundleAsset>,
    model_sha256: String,
}

impl Fixture {
    fn new(root: &Path) -> Self {
        let local_app_data = root.join("LocalFolder");
        fs::create_dir(&local_app_data).expect("MSIX LocalFolder fixture is created");
        let bundle_root = root.join("package");
        fs::create_dir(&bundle_root).expect("package root fixture is created");
        fs::create_dir(bundle_root.join("app")).expect("app folder fixture is created");

        let resource_path = bundle_root.join("app/whisper-desktop.exe");
        fs::write(&resource_path, b"verified desktop fixture")
            .expect("package resource fixture is written");
        let model_path = root.join("model.bin");
        fs::write(&model_path, b"verified model fixture").expect("model fixture is written");
        let bundle_assets = vec![BundleAsset {
            source: PathBuf::from("app/whisper-desktop.exe"),
            destination: PathBuf::from("app/whisper-desktop.exe"),
            sha256: digest_hex(&sha256_file(&resource_path).expect("resource hash succeeds")),
            label: "desktop fixture",
        }];
        let model_sha256 = digest_hex(&sha256_file(&model_path).expect("model hash succeeds"));

        Self {
            local_app_data,
            bundle_root,
            model_path,
            bundle_assets,
            model_sha256,
        }
    }

    fn request(&self) -> InstallRequest {
        InstallRequest {
            local_app_data: Some(self.local_app_data.clone()),
            bundle_root: Some(self.bundle_root.clone()),
            model_path: self.model_path.clone(),
            confirmed: true,
        }
    }

    fn preflight(&self) -> Result<(), InstallError> {
        preflight_bundle_with_assets(
            Some(&self.local_app_data),
            Some(&self.bundle_root),
            &self.model_path,
            &compatible_cpu(),
            &self.bundle_assets,
            &self.model_sha256,
            DestinationRequirement::MayExist,
        )
    }

    fn zip_preflight(&self) -> Result<(), InstallError> {
        preflight_bundle_with_assets(
            Some(&self.local_app_data),
            Some(&self.bundle_root),
            &self.model_path,
            &compatible_cpu(),
            &self.bundle_assets,
            &self.model_sha256,
            DestinationRequirement::MustBeAbsent,
        )
    }

    fn install_path(&self) -> PathBuf {
        self.local_app_data.join("WhisperTurbo")
    }
}

fn compatible_cpu() -> CpuCompatibility {
    CpuCompatibility {
        level: CpuCompatibilityLevel::X86_64V2,
        missing: Vec::new(),
    }
}

fn install_directory_entries(path: &Path) -> Vec<String> {
    let mut entries = fs::read_dir(path)
        .expect("directory can be inspected")
        .map(|entry| {
            entry
                .expect("directory entry is readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

#[test]
fn msix_preflight_accepts_existing_whisper_turbo_data_without_mutation() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::create_dir(fixture.install_path()).expect("existing user data folder is created");
    let owner_marker = fixture.install_path().join("owner.txt");
    fs::write(&owner_marker, b"preserve existing user data").expect("marker is written");
    let entries_before = install_directory_entries(&fixture.install_path());
    let resource_path = fixture.bundle_root.join("app/whisper-desktop.exe");
    let resource_before = fs::read(&resource_path).expect("package resource is readable");
    let model_before = fs::read(&fixture.model_path).expect("model is readable");

    fixture
        .preflight()
        .expect("valid package should pass when user data already exists");

    assert_eq!(
        fs::read(&owner_marker).expect("existing marker remains readable"),
        b"preserve existing user data"
    );
    assert_eq!(
        install_directory_entries(&fixture.install_path()),
        entries_before,
        "read-only preflight does not add or remove user data"
    );
    assert_eq!(
        fs::read(resource_path).expect("package resource remains readable"),
        resource_before,
        "read-only preflight does not alter package resources"
    );
    assert_eq!(
        fs::read(&fixture.model_path).expect("model remains readable"),
        model_before,
        "read-only preflight does not alter the model"
    );
}

#[test]
fn msix_preflight_rejects_an_altered_package_resource() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::create_dir(fixture.install_path()).expect("existing user data folder is created");
    fs::write(
        fixture.bundle_root.join("app/whisper-desktop.exe"),
        b"altered desktop fixture",
    )
    .expect("package resource fixture is altered");

    assert_eq!(
        fixture
            .preflight()
            .expect_err("altered resource is refused"),
        InstallError::ResourceHashMismatch("desktop fixture")
    );
}

#[test]
fn msix_preflight_rejects_an_altered_model() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::create_dir(fixture.install_path()).expect("existing user data folder is created");
    fs::write(&fixture.model_path, b"altered model fixture").expect("model fixture is altered");

    assert_eq!(
        fixture.preflight().expect_err("altered model is refused"),
        InstallError::ModelHashMismatch
    );
}

#[test]
fn zip_preflight_itself_refuses_an_existing_whisper_turbo_destination() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::create_dir(fixture.install_path()).expect("existing install destination is created");
    let owner_marker = fixture.install_path().join("owner.txt");
    fs::write(&owner_marker, b"preserve ZIP destination").expect("marker is written");

    assert_eq!(
        fixture
            .zip_preflight()
            .expect_err("ZIP preflight refuses an existing destination"),
        InstallError::DestinationExists
    );
    assert_eq!(
        fs::read(&owner_marker).expect("existing marker remains readable"),
        b"preserve ZIP destination"
    );
}

#[test]
fn zip_install_still_refuses_an_existing_whisper_turbo_destination() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::create_dir(fixture.install_path()).expect("existing install destination is created");
    let owner_marker = fixture.install_path().join("owner.txt");
    fs::write(&owner_marker, b"preserve ZIP destination").expect("marker is written");

    let error = install_with(
        fixture.request(),
        &compatible_cpu(),
        &fixture.bundle_assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| {},
    )
    .expect_err("ZIP installation refuses an existing destination");

    assert_eq!(error, InstallError::DestinationExists);
    assert_eq!(
        fs::read(&owner_marker).expect("existing marker remains readable"),
        b"preserve ZIP destination"
    );
}
