use super::{
    BundleAsset, InstallError, InstallOutcome, InstallRequest, copy_new_file, digest_hex,
    install_with,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use whisper_adapters::archive::sha256_file;
use whisper_core::cpu_compatibility::{CpuCompatibility, CpuCompatibilityLevel, CpuRequirement};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestArea(PathBuf);

impl TestArea {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "whisper-bootstrap-test-{}-{}",
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
    assets: Vec<BundleAsset>,
    model_sha256: String,
}

impl Fixture {
    fn new(root: &Path) -> Self {
        let local_app_data = root.join("local-app-data");
        fs::create_dir(&local_app_data).expect("local app data fixture is created");
        let bundle_root = root.join("bundle");
        fs::create_dir(&bundle_root).expect("bundle fixture is created");
        fs::create_dir(bundle_root.join("app")).expect("app fixture is created");
        fs::create_dir(bundle_root.join("notices-candidate")).expect("notice fixture is created");
        let model_path = root.join("model.bin");
        fs::write(&model_path, b"verified fixture model").expect("model fixture is written");
        let assets = [
            (
                "app/whisper-desktop.exe",
                "app/whisper-desktop.exe",
                "approved desktop fixture",
                "whisper-desktop.exe",
            ),
            (
                "notices-candidate/LICENSE.md",
                "notices/LICENSE.md",
                "approved notice fixture",
                "LICENSE.md",
            ),
        ]
        .into_iter()
        .map(|(source, destination, contents, label)| {
            let source_path = bundle_root.join(source);
            fs::write(&source_path, contents).expect("bundle resource is written");
            BundleAsset {
                source: PathBuf::from(source),
                destination: PathBuf::from(destination),
                sha256: digest_hex(&sha256_file(&source_path).expect("resource hash succeeds")),
                label,
            }
        })
        .collect();
        let model_sha256 = digest_hex(&sha256_file(&model_path).expect("model hash succeeds"));
        Self {
            local_app_data,
            bundle_root,
            model_path,
            assets,
            model_sha256,
        }
    }

    fn request(&self, confirmed: bool) -> InstallRequest {
        InstallRequest {
            local_app_data: Some(self.local_app_data.clone()),
            bundle_root: Some(self.bundle_root.clone()),
            model_path: self.model_path.clone(),
            confirmed,
        }
    }

    fn install_path(&self) -> PathBuf {
        self.local_app_data.join("WhisperTurbo")
    }
}

fn compatibility(level: CpuCompatibilityLevel) -> CpuCompatibility {
    CpuCompatibility {
        level,
        missing: if level == CpuCompatibilityLevel::BelowX86_64V2 {
            vec![CpuRequirement::Sse42]
        } else {
            Vec::new()
        },
    }
}

fn install(fixture: &Fixture) -> Result<InstallOutcome, InstallError> {
    install_with(
        fixture.request(true),
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| {},
    )
}

fn assert_no_staging_directory(local_app_data: &Path) {
    let leftovers = fs::read_dir(local_app_data)
        .expect("local app data can be inspected")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".WhisperTurbo-stage-")
        })
        .count();
    assert_eq!(leftovers, 0, "owned staging directory was cleaned");
}

#[test]
fn indeterminate_cpu_refuses_the_real_install_path_before_filesystem_mutation() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let request = InstallRequest {
        local_app_data: None,
        bundle_root: None,
        model_path: fixture.model_path.clone(),
        confirmed: true,
    };
    let error = install_with(
        request,
        &compatibility(CpuCompatibilityLevel::Indeterminate),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| {},
    )
    .expect_err("indeterminate CPU is refused");
    assert_eq!(error, InstallError::CpuIndeterminate);
    assert_eq!(
        error.to_string(),
        "Installation refusée : la compatibilité du CPU n'a pas pu être vérifiée."
    );
    assert!(!fixture.install_path().exists());
    assert_no_staging_directory(&fixture.local_app_data);
}

#[test]
fn below_v2_cpu_refuses_before_creating_the_installation_root() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let error = install_with(
        fixture.request(true),
        &compatibility(CpuCompatibilityLevel::BelowX86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| {},
    )
    .expect_err("below-v2 CPU is refused");
    assert_eq!(error, InstallError::CpuUnsupported);
    assert!(!fixture.install_path().exists());
    assert_no_staging_directory(&fixture.local_app_data);
}

#[test]
fn cancellation_returns_before_any_installation_mutation() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let result = install_with(
        fixture.request(false),
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| panic!("cancelled installation must not approach publication"),
    )
    .expect("cancellation is a normal result");
    assert!(matches!(result, InstallOutcome::Cancelled));
    assert!(!fixture.install_path().exists());
    assert_no_staging_directory(&fixture.local_app_data);
}

#[test]
fn missing_and_wrong_model_refuse_without_creating_a_final_root() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let mut request = fixture.request(true);
    request.model_path = area.path().join("missing-model.bin");
    let missing = install_with(
        request,
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| {},
    )
    .expect_err("missing model is refused");
    assert_eq!(missing, InstallError::ModelUnavailable);

    let wrong_hash = "00".repeat(32);
    let wrong = install_with(
        fixture.request(true),
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &wrong_hash,
        copy_new_file,
        |_| {},
    )
    .expect_err("wrong model identity is refused");
    assert_eq!(wrong, InstallError::ModelHashMismatch);
    assert!(!fixture.install_path().exists());
    assert_no_staging_directory(&fixture.local_app_data);
}

#[test]
fn missing_bundle_resource_refuses_before_creating_a_final_root() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::remove_file(fixture.bundle_root.join("app/whisper-desktop.exe"))
        .expect("fixture resource is removed");
    let error = install(&fixture).expect_err("missing resource is refused");
    assert_eq!(
        error,
        InstallError::ResourceUnavailable("whisper-desktop.exe")
    );
    assert!(!fixture.install_path().exists());
    assert_no_staging_directory(&fixture.local_app_data);
}

#[test]
fn altered_resource_copy_is_detected_and_never_published() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let error = install_with(
        fixture.request(true),
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        |source, destination| {
            copy_new_file(source, destination)?;
            if destination
                .file_name()
                .is_some_and(|name| name.to_string_lossy() == "whisper-desktop.exe")
            {
                OpenOptions::new()
                    .append(true)
                    .open(destination)?
                    .write_all(b" altered during copy")?;
            }
            Ok(())
        },
        |_| {},
    )
    .expect_err("altered copy is refused");
    let InstallError::StagingPreserved(staging, detail) = error else {
        panic!("altered copy and preserved staging are reported");
    };
    assert!(detail.contains("empreinte de ressource incorrecte") || detail.contains("altérée"));
    assert!(detail.contains("identité du répertoire non vérifiable"));
    assert!(
        staging.is_dir(),
        "unverified stage remains available for diagnosis"
    );
    assert!(!fixture.install_path().exists());
}

#[test]
fn successful_install_publishes_verified_model_resources_and_notices_together() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let installed = install(&fixture).expect("verified installation succeeds");
    let InstallOutcome::Installed(path) = installed else {
        panic!("confirmed installation is published");
    };
    assert_eq!(
        fs::canonicalize(&path).expect("installed path canonicalizes"),
        fs::canonicalize(fixture.install_path()).expect("fixture path canonicalizes")
    );
    assert_eq!(
        fs::read(path.join("app/whisper-desktop.exe")).expect("installed resource is readable"),
        b"approved desktop fixture"
    );
    assert_eq!(
        fs::read(path.join("models/ggml-large-v3-turbo.bin")).expect("installed model is readable"),
        b"verified fixture model"
    );
    assert_eq!(
        fs::read(path.join("notices/LICENSE.md")).expect("installed notice is readable"),
        b"approved notice fixture"
    );
    assert!(!path.join("settings").exists());
    assert!(!path.join("logs").exists());
    assert_no_staging_directory(&fixture.local_app_data);
}

#[test]
fn a_simultaneously_created_destination_is_preserved_without_overwrite() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let final_path = fixture.install_path();
    let (created_sender, created_receiver) = mpsc::sync_channel(0);
    let (finished_sender, finished_receiver) = mpsc::sync_channel(0);
    let concurrent_path = final_path.clone();
    let competitor = thread::spawn(move || {
        created_receiver
            .recv()
            .expect("installer reaches the publication boundary");
        fs::create_dir(&concurrent_path).expect("concurrent install reserves the final root");
        finished_sender
            .send(())
            .expect("installer waits for the concurrent root");
    });
    let error = install_with(
        fixture.request(true),
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |_| {
            created_sender
                .send(())
                .expect("concurrent actor is started");
            finished_receiver
                .recv()
                .expect("concurrent root is ready before commit");
        },
    )
    .expect_err("exclusive publication refuses the concurrent destination");
    competitor.join().expect("concurrent actor completes");
    let InstallError::DestinationExistsWithPreservedStaging(staging) = error else {
        panic!("empty concurrent root must be refused and staging reported");
    };
    assert_eq!(
        fs::canonicalize(staging.parent().expect("staging has a parent"))
            .expect("staging parent canonicalizes"),
        fs::canonicalize(&fixture.local_app_data).expect("fixture local app data canonicalizes")
    );
    assert!(
        fs::read_dir(&final_path)
            .expect("concurrent root remains readable")
            .next()
            .is_none(),
        "concurrent empty root remains empty"
    );
    assert!(!final_path.join("app").exists());
    assert!(!final_path.join("models").exists());
    assert!(!final_path.join("notices").exists());
    assert!(staging.is_dir(), "unverified temporary is preserved");
}

#[test]
fn replaced_staging_directory_is_preserved_and_never_published() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    let final_path = fixture.install_path();
    let error = install_with(
        fixture.request(true),
        &compatibility(CpuCompatibilityLevel::X86_64V2),
        &fixture.assets,
        &fixture.model_sha256,
        copy_new_file,
        |staging| {
            fs::remove_dir_all(staging).expect("original stage is removed by test actor");
            fs::create_dir(staging).expect("replacement occupies staging path");
            fs::write(staging.join("third-party-marker"), b"preserve me")
                .expect("replacement marker is written");
        },
    )
    .expect_err("modified staging tree is refused before publication");
    let InstallError::StagingPreserved(staging, detail) = error else {
        panic!("refusal reports the preserved replacement directory");
    };
    assert!(detail.contains("arborescence temporaire"));
    assert_eq!(
        fs::read(staging.join("third-party-marker")).expect("replacement marker survives"),
        b"preserve me"
    );
    assert!(!final_path.exists(), "unverified staging is not published");
}

#[test]
fn an_existing_destination_is_refused_before_the_bundle_is_copied() {
    let area = TestArea::new();
    let fixture = Fixture::new(area.path());
    fs::create_dir(fixture.install_path()).expect("pre-existing install root is created");
    fs::write(fixture.install_path().join("owner.txt"), b"existing owner")
        .expect("existing marker is written");
    let error = install(&fixture).expect_err("pre-existing destination is refused");
    assert_eq!(error, InstallError::DestinationExists);
    assert_eq!(
        fs::read(fixture.install_path().join("owner.txt"))
            .expect("existing marker remains readable"),
        b"existing owner"
    );
    assert_no_staging_directory(&fixture.local_app_data);
}
