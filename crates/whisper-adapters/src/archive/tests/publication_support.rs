#[cfg(test)]
const APPROVED_MODEL_SIZE: u64 = 1_624_555_275;

#[cfg(test)]
const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

#[cfg(test)]
#[derive(Default)]
struct PublicationCopyFault {
    root: Option<PathBuf>,
    cut_after_extension: Option<String>,
}

#[cfg(test)]
static PUBLICATION_COPY_FAULT: std::sync::OnceLock<std::sync::Mutex<PublicationCopyFault>> =
    std::sync::OnceLock::new();

#[cfg(test)]
pub(crate) fn inject_copy_fallback_publication_cut(root: &Path, extension: &str) {
    let fault = PUBLICATION_COPY_FAULT.get_or_init(Default::default);
    let mut fault = fault.lock().expect("publication fault mutex");
    fault.root = Some(root.to_path_buf());
    fault.cut_after_extension = Some(extension.to_owned());
}

#[cfg(test)]
pub(crate) fn clear_copy_fallback_publication_cut() {
    if let Some(fault) = PUBLICATION_COPY_FAULT.get() {
        *fault.lock().expect("publication fault mutex") = PublicationCopyFault::default();
    }
}

#[cfg(test)]
fn publication_copy_fault_applies(root: &Path) -> bool {
    PUBLICATION_COPY_FAULT
        .get()
        .and_then(|fault| fault.lock().ok())
        .is_some_and(|fault| fault.root.as_deref() == Some(root))
}

#[cfg(test)]
fn publication_copy_fault_after(root: &Path, extension: &str) -> Result<(), String> {
    let Some(fault) = PUBLICATION_COPY_FAULT.get() else {
        return Ok(());
    };
    let mut fault = fault
        .lock()
        .map_err(|_| "test publication fault mutex poisoned".to_owned())?;
    if fault.root.as_deref() == Some(root)
        && fault.cut_after_extension.as_deref() == Some(extension)
    {
        fault.cut_after_extension = None;
        return Err(format!(
            "test interruption after final {extension} create_new"
        ));
    }
    Ok(())
}
