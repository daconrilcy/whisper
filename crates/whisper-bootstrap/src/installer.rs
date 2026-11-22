use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use whisper_adapters::archive::sha256_file;
use whisper_core::cpu_compatibility::{CpuCompatibility, CpuCompatibilityLevel};

mod assets;
mod preflight;
pub(crate) use preflight::{bundled_turbo_model, preflight_bundle};

const INSTALL_DIRECTORY: &str = "WhisperTurbo";
const MODEL_FILE: &str = "ggml-large-v3-turbo.bin";
const MODEL_SHA256: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
pub(crate) const DUAL_EDITION: bool =
    option_env!("WHISPER_DISTRIBUTION_EDITION_DUAL").is_some();
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct BundleAsset {
    source: PathBuf,
    destination: PathBuf,
    sha256: String,
    label: &'static str,
}

pub(crate) struct InstallRequest {
    pub(crate) local_app_data: Option<PathBuf>,
    pub(crate) bundle_root: Option<PathBuf>,
    pub(crate) model_path: PathBuf,
    pub(crate) confirmed: bool,
}

#[derive(Debug)]
pub(crate) enum InstallOutcome {
    Installed(PathBuf),
    Cancelled,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InstallError {
    CpuUnsupported,
    CpuIndeterminate,
    LocalAppDataUnavailable,
    BundleUnavailable,
    DestinationExists,
    ModelUnavailable,
    ModelHashMismatch,
    ResourceUnavailable(&'static str),
    ResourceHashMismatch(&'static str),
    ResourceCopyAltered(&'static str),
    Io(&'static str, String),
    DestinationExistsWithPreservedStaging(PathBuf),
    StagingPreserved(PathBuf, String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::CpuUnsupported => {
                "Installation refusée : le CPU ne satisfait pas le niveau x86-64-v2.".to_owned()
            }
            Self::CpuIndeterminate => {
                "Installation refusée : la compatibilité du CPU n'a pas pu être vérifiée."
                    .to_owned()
            }
            Self::LocalAppDataUnavailable => {
                "Installation refusée : %LOCALAPPDATA% est absent ou inaccessible.".to_owned()
            }
            Self::BundleUnavailable => {
                "Installation refusée : les ressources du paquet sont absentes ou invalides."
                    .to_owned()
            }
            Self::DestinationExists => {
                "Installation refusée : la racine WhisperTurbo existe déjà ou a été créée concurremment; aucune donnée n'a été écrasée.".to_owned()
            }
            Self::DestinationExistsWithPreservedStaging(path) => format!(
                "Installation refusée : la racine WhisperTurbo existe déjà ou a été créée concurremment; le temporaire est conservé en {}.",
                path.display()
            ),
            Self::ModelUnavailable => {
                "Installation refusée : le modèle est absent ou illisible.".to_owned()
            }
            Self::ModelHashMismatch => {
                "Installation refusée : l'empreinte du modèle ou de sa copie est incorrecte."
                    .to_owned()
            }
            Self::ResourceUnavailable(name) => {
                format!("Installation refusée : ressource absente ou illisible ({name}).")
            }
            Self::ResourceHashMismatch(name) => {
                format!("Installation refusée : empreinte de ressource incorrecte ({name}).")
            }
            Self::ResourceCopyAltered(name) => {
                format!("Installation refusée : la copie de la ressource a été altérée ({name}).")
            }
            Self::Io(operation, detail) => format!("Échec pendant {operation} : {detail}"),
            Self::StagingPreserved(path, detail) => format!(
                "Le temporaire d'installation est conservé en {} : {detail}",
                path.display()
            ),
        };
        formatter.write_str(&message)
    }
}

pub(crate) fn install(
    request: InstallRequest,
    compatibility: &CpuCompatibility,
) -> Result<InstallOutcome, InstallError> {
    let assets = assets::bundle_assets();
    install_with(
        request,
        compatibility,
        &assets,
        MODEL_SHA256,
        copy_new_file,
        |_| {},
    )
}

fn install_with<C, H>(
    request: InstallRequest,
    compatibility: &CpuCompatibility,
    assets: &[BundleAsset],
    model_sha256: &str,
    copy_file: C,
    before_publish: H,
) -> Result<InstallOutcome, InstallError>
where
    C: Fn(&Path, &Path) -> io::Result<()>,
    H: FnOnce(&Path),
{
    require_compatible_cpu(compatibility)?;
    if !request.confirmed {
        return Ok(InstallOutcome::Cancelled);
    }
    let inputs = validate_inputs(&request, assets, model_sha256)?;
    let staging = create_staging_directory(&inputs.local_app_data)?;
    if let Err(error) = prepare_installation(&inputs, &staging, assets, copy_file) {
        let original = error.to_string();
        return match report_staging_preserved(&staging, &inputs.local_app_data) {
            Ok(()) => Err(error),
            Err(InstallError::StagingPreserved(path, detail)) => Err(
                InstallError::StagingPreserved(path, format!("{original}; {detail}")),
            ),
            Err(cleanup_error) => Err(cleanup_error),
        };
    }
    before_publish(&staging);
    if let Err(detail) = verify_staged_installation(&staging, assets, &inputs.model_sha256) {
        return Err(InstallError::StagingPreserved(staging, detail));
    }
    publish_installation(&staging, &inputs.install_path)
}

struct ValidatedInputs {
    local_app_data: PathBuf,
    bundle_root: PathBuf,
    model_path: PathBuf,
    model_sha256: String,
    install_path: PathBuf,
}

fn require_compatible_cpu(compatibility: &CpuCompatibility) -> Result<(), InstallError> {
    if super::preflight_message(compatibility).is_ok() {
        return Ok(());
    }
    match compatibility.level {
        CpuCompatibilityLevel::BelowX86_64V2 => Err(InstallError::CpuUnsupported),
        CpuCompatibilityLevel::Indeterminate => Err(InstallError::CpuIndeterminate),
        CpuCompatibilityLevel::X86_64V2 => unreachable!("preflight admits x86-64-v2"),
    }
}

fn validate_inputs(
    request: &InstallRequest,
    assets: &[BundleAsset],
    model_sha256: &str,
) -> Result<ValidatedInputs, InstallError> {
    let local_app_data = request
        .local_app_data
        .as_deref()
        .ok_or(InstallError::LocalAppDataUnavailable)?;
    let local_app_data =
        existing_directory(local_app_data).map_err(|_| InstallError::LocalAppDataUnavailable)?;
    let install_path = local_app_data.join(INSTALL_DIRECTORY);
    if path_exists(&install_path)
        .map_err(|error| InstallError::Io("le contrôle de destination", error.to_string()))?
    {
        return Err(InstallError::DestinationExists);
    }

    let bundle_root = request
        .bundle_root
        .as_deref()
        .ok_or(InstallError::BundleUnavailable)?;
    let bundle_root =
        existing_directory(bundle_root).map_err(|_| InstallError::BundleUnavailable)?;
    validate_assets(&bundle_root, assets)?;
    let model_path = validate_model(&request.model_path, model_sha256)?;
    Ok(ValidatedInputs {
        local_app_data,
        bundle_root,
        model_path,
        model_sha256: model_sha256.to_owned(),
        install_path,
    })
}

fn validate_assets(bundle_root: &Path, assets: &[BundleAsset]) -> Result<(), InstallError> {
    for asset in assets {
        if !is_safe_relative(&asset.source) || !is_safe_relative(&asset.destination) {
            return Err(InstallError::ResourceUnavailable(
                "chemin de paquet invalide",
            ));
        }
        let source = bundle_root.join(&asset.source);
        if !regular_file(&source) {
            return Err(InstallError::ResourceUnavailable(asset.label));
        }
        let digest =
            sha256_file(&source).map_err(|_| InstallError::ResourceUnavailable(asset.label))?;
        if digest_hex(&digest) != asset.sha256 {
            return Err(InstallError::ResourceHashMismatch(asset.label));
        }
    }
    Ok(())
}

fn validate_model(path: &Path, expected_sha256: &str) -> Result<PathBuf, InstallError> {
    if !path.is_absolute() || !regular_file(path) {
        return Err(InstallError::ModelUnavailable);
    }
    let digest = sha256_file(path).map_err(|_| InstallError::ModelUnavailable)?;
    if digest_hex(&digest) != expected_sha256 {
        return Err(InstallError::ModelHashMismatch);
    }
    Ok(path.to_path_buf())
}

fn prepare_installation<C>(
    inputs: &ValidatedInputs,
    staging: &Path,
    assets: &[BundleAsset],
    copy_file: C,
) -> Result<(), InstallError>
where
    C: Fn(&Path, &Path) -> io::Result<()>,
{
    create_staging_layout(staging)?;
    copy_verified(
        &inputs.model_path,
        &staging.join("models").join(MODEL_FILE),
        &inputs.model_sha256,
        "modèle",
        &copy_file,
        |_| InstallError::ModelHashMismatch,
    )?;
    for asset in assets {
        let source = inputs.bundle_root.join(&asset.source);
        let destination = staging.join(&asset.destination);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                InstallError::Io("la préparation des ressources", error.to_string())
            })?;
        }
        copy_verified(
            &source,
            &destination,
            &asset.sha256,
            asset.label,
            &copy_file,
            |name| InstallError::ResourceCopyAltered(name),
        )?;
    }
    Ok(())
}

fn create_staging_layout(path: &Path) -> Result<(), InstallError> {
    for directory in ["app", "models", "notices"] {
        fs::create_dir(path.join(directory)).map_err(|error| {
            InstallError::Io("la préparation du répertoire temporaire", error.to_string())
        })?;
    }
    Ok(())
}

fn copy_verified<C, F>(
    source: &Path,
    destination: &Path,
    expected_sha256: &str,
    label: &'static str,
    copy_file: &C,
    mismatch: F,
) -> Result<(), InstallError>
where
    C: Fn(&Path, &Path) -> io::Result<()>,
    F: Fn(&'static str) -> InstallError,
{
    copy_file(source, destination)
        .map_err(|error| InstallError::Io("la copie d'une ressource", error.to_string()))?;
    let digest = sha256_file(destination).map_err(|_| mismatch(label))?;
    if digest_hex(&digest) != expected_sha256 {
        return Err(mismatch(label));
    }
    Ok(())
}

fn publish_installation(
    staging: &Path,
    install_path: &Path,
) -> Result<InstallOutcome, InstallError> {
    match renamest::rename_exclusive(staging, install_path) {
        Ok(()) => Ok(InstallOutcome::Installed(install_path.to_path_buf())),
        Err(error) => {
            let destination_exists = match path_exists(install_path) {
                Ok(exists) => exists,
                Err(check) => {
                    return Err(InstallError::StagingPreserved(
                        staging.to_path_buf(),
                        format!("publication refusée ({check}); temporaire conservé"),
                    ));
                }
            };
            if destination_exists || error.kind() == io::ErrorKind::AlreadyExists {
                Err(InstallError::DestinationExistsWithPreservedStaging(
                    staging.to_path_buf(),
                ))
            } else {
                Err(InstallError::StagingPreserved(
                    staging.to_path_buf(),
                    format!("publication refusée ({error}); temporaire conservé"),
                ))
            }
        }
    }
}

fn verify_staged_installation(
    staging: &Path,
    assets: &[BundleAsset],
    model_sha256: &str,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(staging).map_err(|error| error.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("le répertoire temporaire n'est plus un répertoire simple".to_owned());
    }
    let expected = expected_staging_entries(assets);
    let mut actual = BTreeMap::new();
    collect_staging_entries(staging, staging, &mut actual)?;
    if actual != expected {
        return Err("l'arborescence temporaire diffère du paquet préparé".to_owned());
    }
    verify_staged_file(staging, Path::new("models").join(MODEL_FILE), model_sha256)?;
    for asset in assets {
        verify_staged_file(staging, asset.destination.clone(), &asset.sha256)?;
    }
    Ok(())
}

fn expected_staging_entries(assets: &[BundleAsset]) -> BTreeMap<PathBuf, bool> {
    let mut entries = BTreeMap::new();
    for directory in ["app", "models", "notices"] {
        entries.insert(PathBuf::from(directory), true);
    }
    let mut files = assets
        .iter()
        .map(|asset| asset.destination.clone())
        .collect::<Vec<_>>();
    files.push(Path::new("models").join(MODEL_FILE));
    for file in files {
        let mut parent = file.parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            entries.insert(path.to_path_buf(), true);
            parent = path.parent();
        }
        entries.insert(file, false);
    }
    entries
}

fn collect_staging_entries(
    staging: &Path,
    directory: &Path,
    entries: &mut BTreeMap<PathBuf, bool>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let relative = path
            .strip_prefix(staging)
            .map_err(|error| error.to_string())?
            .to_path_buf();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "lien interdit dans le temporaire: {}",
                relative.display()
            ));
        }
        if metadata.is_dir() {
            entries.insert(relative, true);
            collect_staging_entries(staging, &path, entries)?;
        } else if metadata.is_file() {
            entries.insert(relative, false);
        } else {
            return Err(format!(
                "type de ressource inattendu: {}",
                relative.display()
            ));
        }
    }
    Ok(())
}

fn verify_staged_file(staging: &Path, relative: PathBuf, expected: &str) -> Result<(), String> {
    let path = staging.join(&relative);
    if !regular_file(&path) {
        return Err(format!(
            "ressource absente ou non régulière: {}",
            relative.display()
        ));
    }
    let digest = sha256_file(&path).map_err(|error| error.to_string())?;
    if digest_hex(&digest) != expected {
        return Err(format!(
            "empreinte temporaire incorrecte: {}",
            relative.display()
        ));
    }
    Ok(())
}

fn existing_directory(path: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a plain directory",
        ));
    }
    fs::canonicalize(path)
}

fn regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn path_exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn create_staging_directory(local_app_data: &Path) -> Result<PathBuf, InstallError> {
    for _ in 0..8 {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = local_app_data.join(format!(
            ".WhisperTurbo-stage-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(InstallError::Io(
                    "la création du répertoire temporaire",
                    error.to_string(),
                ));
            }
        }
    }
    Err(InstallError::Io(
        "la réservation du répertoire temporaire",
        "huit noms consécutifs étaient déjà utilisés".to_owned(),
    ))
}

fn report_staging_preserved(path: &Path, local_app_data: &Path) -> Result<(), InstallError> {
    if path.parent() != Some(local_app_data)
        || !path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".WhisperTurbo-stage-"))
    {
        return Err(InstallError::StagingPreserved(
            path.to_path_buf(),
            "chemin inattendu; temporaire conservé".to_owned(),
        ));
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(InstallError::StagingPreserved(
            path.to_path_buf(),
            error.to_string(),
        )),
        Ok(_) => Err(InstallError::StagingPreserved(
            path.to_path_buf(),
            "identité du répertoire non vérifiable; temporaire conservé".to_owned(),
        )),
    }
}

fn copy_new_file(source: &Path, destination: &Path) -> io::Result<()> {
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    io::copy(&mut input, &mut output)?;
    output.flush()?;
    output.sync_all()
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn digest_hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
