use std::io;
use std::path::{Component, Path, PathBuf};

#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChildState {
    NotStarted,
    RunningOrUnknown,
    Finished,
}

#[cfg(windows)]
struct TemporaryTestRoot {
    path: PathBuf,
    temp_root: PathBuf,
    child: ChildState,
    fail_cleanup_for_test: bool,
}

#[cfg(windows)]
impl TemporaryTestRoot {
    fn create(path: PathBuf) -> io::Result<Self> {
        let temp = std::fs::canonicalize(std::env::temp_dir())?;
        validate_owned_path(&path, &temp)?;
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "test root has no parent")
        })?;
        validate_owned_path(parent, &temp)?;
        std::fs::create_dir_all(parent)?;
        validate_owned_path(&path, &temp)?;
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if let Err(error) = validate_owned_path(&path, &temp) {
            report_diagnostic(
                std::io::stderr(),
                &format!(
                    "test root kept after unsafe post-create path check: {}: {error}",
                    path.display()
                ),
            );
            return Err(error);
        }
        Ok(Self {
            path,
            temp_root: temp,
            child: ChildState::NotStarted,
            fail_cleanup_for_test: false,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn child_starting(&mut self) {
        self.child = ChildState::RunningOrUnknown;
    }

    fn child_finished(&mut self) {
        self.child = ChildState::Finished;
    }

    fn preserve_if_child_unknown(&mut self) {
        self.child = ChildState::RunningOrUnknown;
    }

    fn refuse_cleanup_for_test(&mut self) {
        self.fail_cleanup_for_test = true;
    }

    fn try_cleanup(&self) -> io::Result<()> {
        validate_owned_path(&self.path, &self.temp_root)?;
        std::fs::remove_dir_all(&self.path)
    }
}

#[cfg(windows)]
impl Drop for TemporaryTestRoot {
    fn drop(&mut self) {
        if std::thread::panicking() {
            report_drop_diagnostic(&format!(
                "test root retained while unwinding after panic: {}",
                self.path.display()
            ));
            return;
        }
        if self.child == ChildState::RunningOrUnknown {
            report_diagnostic(
                std::io::stderr(),
                &format!(
                    "test root retained because child completion is unknown: {}",
                    self.path.display()
                ),
            );
            return;
        }
        let cleanup = if self.fail_cleanup_for_test {
            Err(permission_denied("synthetic cleanup refusal"))
        } else {
            self.try_cleanup()
        };
        if let Err(error) = cleanup {
            report_drop_diagnostic(
                &format!(
                    "test root retained after cleanup failure: {}: {error}",
                    self.path.display()
                ),
            );
        }
    }
}

#[cfg(windows)]
fn validate_owned_path(path: &Path, temp: &Path) -> io::Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(permission_denied("test root path is not a normalized absolute path"));
    }
    assert_no_reparse_ancestors(path)?;
    let resolved = confined_path(path, temp)?;
    if resolved == temp || !resolved.starts_with(temp) {
        return Err(permission_denied("test root resolves outside TEMP"));
    }
    Ok(())
}

#[cfg(windows)]
fn confined_path(path: &Path, temp: &Path) -> io::Result<PathBuf> {
    let mut cursor = path;
    let mut missing = Vec::new();
    let existing = loop {
        match std::fs::canonicalize(cursor) {
            Ok(resolved) => break resolved,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let name = cursor.file_name().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "test root has no existing ancestor")
                })?;
                missing.push(name.to_os_string());
                cursor = cursor.parent().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "test root has no existing ancestor")
                })?;
            }
            Err(error) => return Err(error),
        }
    };
    if !existing.starts_with(temp) {
        return Err(permission_denied("test root resolves outside TEMP"));
    }
    let mut resolved = existing;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    if !resolved.starts_with(temp) {
        return Err(permission_denied("test root resolves outside TEMP"));
    }
    Ok(resolved)
}

#[cfg(windows)]
fn permission_denied(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

#[cfg(windows)]
fn report_diagnostic(mut output: impl std::io::Write, message: &str) {
    use std::io::Write as _;

    let _ = writeln!(output, "{message}");
}

#[cfg(windows)]
thread_local! {
    static DROP_DIAGNOSTIC_CAPTURE: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(windows)]
fn report_drop_diagnostic(message: &str) {
    let captured = DROP_DIAGNOSTIC_CAPTURE
        .try_with(|capture| {
            let mut capture = capture.borrow_mut();
            if let Some(output) = capture.as_mut() {
                report_diagnostic(output, message);
                true
            } else {
                false
            }
        })
        .unwrap_or(false);
    if !captured {
        report_diagnostic(std::io::stderr(), message);
    }
}

#[cfg(windows)]
fn begin_drop_diagnostic_capture() {
    let _ = DROP_DIAGNOSTIC_CAPTURE.try_with(|capture| *capture.borrow_mut() = Some(Vec::new()));
}

#[cfg(windows)]
fn take_drop_diagnostic_capture() -> Vec<u8> {
    DROP_DIAGNOSTIC_CAPTURE
        .try_with(|capture| capture.borrow_mut().take().unwrap_or_default())
        .unwrap_or_default()
}

#[cfg(windows)]
fn assert_no_reparse_ancestors(path: &Path) -> io::Result<()> {
    use std::os::windows::fs::MetadataExt;

    let mut cursor = Some(path);
    while let Some(current) = cursor {
        match std::fs::symlink_metadata(current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("test root path crosses a symbolic link: {}", current.display()),
                ));
            }
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("test root path crosses a reparse point: {}", current.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        cursor = current.parent();
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn temporary_test_root_requires_a_new_confined_path() {
    let existing = test_artifact_root("n1-root-existing");
    create_test_fixture_dir(&existing).unwrap();
    assert!(TemporaryTestRoot::create(existing.clone()).is_err());
    std::fs::remove_dir(&existing).unwrap();

    let outside = std::env::current_exe().unwrap();
    assert!(TemporaryTestRoot::create(outside).is_err());
}

#[cfg(windows)]
#[test]
fn temporary_test_root_rejects_parent_traversal_without_touching_sibling() {
    let temp = std::env::temp_dir();
    let parent = temp.parent().unwrap();
    let sibling_name = test_artifact_root("n1-root-sibling-witness")
        .file_name()
        .unwrap()
        .to_os_string();
    let sibling = parent.join(&sibling_name);
    std::fs::create_dir(&sibling).unwrap();
    let marker = sibling.join("marker.txt");
    std::fs::write(&marker, b"must remain intact").unwrap();
    let candidate = temp.join("..").join(&sibling_name).join("child");
    assert!(TemporaryTestRoot::create(candidate).is_err());
    assert_eq!(std::fs::read(&marker).unwrap(), b"must remain intact");
    std::fs::remove_file(marker).unwrap();
    std::fs::remove_dir(sibling).unwrap();
}

#[cfg(windows)]
#[test]
fn temporary_test_root_preserves_root_when_child_completion_is_unknown() {
    let path = test_artifact_root("n1-root-child");
    let mut root = TemporaryTestRoot::create(path.clone()).unwrap();
    root.preserve_if_child_unknown();
    drop(root);
    assert!(path.is_dir());
    std::fs::remove_dir(path).unwrap();
}

#[cfg(windows)]
#[test]
fn temporary_test_root_removal_does_not_follow_an_internal_junction() {
    use std::os::windows::process::CommandExt;

    let root_path = test_artifact_root("n1-root-junction-root");
    let target = test_artifact_root("n1-root-junction-target");
    create_test_fixture_dir(&target).unwrap();
    let marker = target.join("marker.txt");
    std::fs::write(&marker, b"outside target").unwrap();
    let before = sha256(&marker);
    let root = TemporaryTestRoot::create(root_path.clone()).unwrap();
    let junction = root.path().join("junction");
    let result = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert!(result.status.success(), "junction creation failed: {}", String::from_utf8_lossy(&result.stderr));
    drop(root);
    assert!(!root_path.exists(), "owned test root was not removed");
    assert!(marker.is_file(), "cleanup followed the junction target");
    assert_eq!(sha256(&marker), before, "outside target content changed");
    std::fs::remove_file(marker).unwrap();
    std::fs::remove_dir(target).unwrap();
}

#[cfg(windows)]
#[test]
fn temporary_test_root_refuses_a_reparse_ancestor() {
    use std::os::windows::process::CommandExt;

    let target = test_artifact_root("n1-root-ancestor-target");
    let junction = test_artifact_root("n1-root-ancestor-junction");
    create_test_fixture_dir(&target).unwrap();
    let result = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert!(result.status.success(), "junction creation failed: {}", String::from_utf8_lossy(&result.stderr));
    assert!(TemporaryTestRoot::create(junction.join("child")).is_err());
    std::fs::remove_dir(&junction).unwrap();
    std::fs::remove_dir(target).unwrap();
}

#[cfg(windows)]
#[test]
fn temporary_test_root_cleanup_reports_missing_root() {
    let path = test_artifact_root("n1-root-missing");
    let root = TemporaryTestRoot::create(path.clone()).unwrap();
    std::fs::remove_dir(&path).unwrap();
    assert_eq!(root.try_cleanup().unwrap_err().kind(), io::ErrorKind::NotFound);
    let result = std::panic::catch_unwind(|| drop(root));
    assert!(result.is_ok(), "cleanup failure must not panic from Drop");
}

#[cfg(windows)]
#[test]
fn temporary_test_root_cleans_after_child_exits_unsuccessfully() {
    use std::os::windows::process::CommandExt;

    let path = test_artifact_root("n1-root-failed-child");
    let mut root = TemporaryTestRoot::create(path.clone()).unwrap();
    root.child_starting();
    let output = std::process::Command::new("cmd.exe")
        .args(["/C", "exit", "7"])
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    root.child_finished();
    assert_eq!(output.status.code(), Some(7));
    drop(root);
    assert!(!path.exists());
}

#[cfg(windows)]
#[test]
fn temporary_test_root_preserves_during_unwind_after_child_completion() {
    let path = test_artifact_root("n1-root-panic-finished-child");
    let result = std::panic::catch_unwind(|| {
        let mut root = TemporaryTestRoot::create(path.clone()).unwrap();
        root.child_starting();
        root.child_finished();
        panic!("synthetic parent assertion panic after child completion");
    });
    assert!(result.is_err());
    assert!(path.is_dir());
    std::fs::remove_dir_all(&path).unwrap();
}

#[cfg(windows)]
#[test]
fn temporary_test_roots_are_independently_owned() {
    let first_path = test_artifact_root("n1-root-first-owner");
    let second_path = test_artifact_root("n1-root-second-owner");
    let first = TemporaryTestRoot::create(first_path.clone()).unwrap();
    let second = TemporaryTestRoot::create(second_path.clone()).unwrap();
    drop(first);
    assert!(!first_path.exists());
    assert!(second_path.is_dir());
    drop(second);
    assert!(!second_path.exists());
}

#[cfg(windows)]
#[test]
fn temporary_test_root_diagnostic_failure_does_not_panic() {
    struct FailingWriter;
    impl std::io::Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("synthetic stderr failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("synthetic stderr failure"))
        }
    }

    let result = std::panic::catch_unwind(|| {
        report_diagnostic(FailingWriter, "cleanup diagnostic");
    });
    assert!(result.is_ok());
}

#[cfg(windows)]
#[test]
fn temporary_test_root_diagnostic_contains_the_retained_path() {
    let path = test_artifact_root("n1-root-diagnostic-path");
    let mut output = Vec::new();
    report_diagnostic(&mut output, &format!("test root retained: {}", path.display()));
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("test root retained:"));
    assert!(output.contains(&path.to_string_lossy().to_string()));
}

#[cfg(windows)]
fn sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};

    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

#[cfg(windows)]
fn create_test_fixture_dir(path: &Path) -> io::Result<()> {
    let temp = std::fs::canonicalize(std::env::temp_dir())?;
    validate_owned_path(path, &temp)?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "fixture directory has no parent")
    })?;
    validate_owned_path(parent, &temp)?;
    std::fs::create_dir_all(parent)?;
    validate_owned_path(path, &temp)?;
    std::fs::create_dir(path)?;
    validate_owned_path(path, &temp)
}

#[cfg(windows)]
#[test]
fn temporary_test_root_reports_retention_during_active_unwind() {
    use std::panic::AssertUnwindSafe;

    let path = test_artifact_root("n1-root-refusal-during-unwind");
    let mut root = TemporaryTestRoot::create(path.clone()).unwrap();
    root.child_finished();
    root.refuse_cleanup_for_test();
    begin_drop_diagnostic_capture();
    let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _root = root;
        panic!("synthetic parent assertion panic");
    }));
    let diagnostic = String::from_utf8(take_drop_diagnostic_capture()).unwrap();
    let panic = outcome.expect_err("the synthetic parent panic must be observed");
    assert_eq!(panic.downcast_ref::<&str>(), Some(&"synthetic parent assertion panic"));
    assert!(path.is_dir(), "refused cleanup must retain the owned root");
    assert!(diagnostic.contains("retained while unwinding"));
    assert!(diagnostic.contains(&path.to_string_lossy().to_string()));
    std::fs::remove_dir_all(path).unwrap();
}
