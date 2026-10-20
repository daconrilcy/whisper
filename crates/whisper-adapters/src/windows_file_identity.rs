#![allow(unsafe_code)]

//! Narrow handle-based identity and disposition operations for RTC-02 cleanup.
//! No path-based delete API is used here.

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_DISPOSITION_INFO,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_ID_INFO,
        FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FileAttributeTagInfo, FileDispositionInfo,
        FileIdInfo, GetFileInformationByHandleEx, SetFileInformationByHandle,
    },
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct FileIdentity {
    pub volume_serial: u64,
    pub file_id: [u8; 16],
    pub is_directory: bool,
}

pub(crate) struct VerifiedFile {
    file: File,
    pub identity: FileIdentity,
}

pub(crate) struct VerifiedSource {
    _file: File,
    pub identity: FileIdentity,
    pub sha256: [u8; 32],
}

impl VerifiedFile {
    pub fn open(path: &Path, directory: bool) -> Result<Self, String> {
        let flags = FILE_FLAG_OPEN_REPARSE_POINT.0
            | if directory {
                FILE_FLAG_BACKUP_SEMANTICS.0
            } else {
                0
            };
        let file = OpenOptions::new()
            .access_mode(
                DELETE.0 | FILE_READ_ATTRIBUTES.0 | if directory { 0 } else { FILE_GENERIC_READ.0 },
            )
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(flags)
            .open(path)
            .map_err(|error| {
                format!(
                    "StorageUnavailable: ouvrir l'objet exact {}: {error}",
                    path.display()
                )
            })?;
        let identity = identity_for_handle(&file, directory)?;
        Ok(Self { file, identity })
    }

    pub fn verify(&self, expected: &FileIdentity) -> Result<(), String> {
        let actual = identity_for_handle(&self.file, expected.is_directory)?;
        if &actual != expected {
            return Err("CleanupIdentityChanged: l'identifiant Windows du fichier a changé".into());
        }
        Ok(())
    }

    pub fn sha256(&mut self) -> Result<String, String> {
        use std::io::{Seek, SeekFrom};
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("CleanupIdentityChanged: repositionner l'objet: {error}"))?;
        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = self.file.read(&mut buffer).map_err(|error| {
                format!("CleanupIdentityChanged: relire l'objet vérifié: {error}")
            })?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Ok(sha256_hex(&hasher.finalize()))
    }

    pub fn read_all(&mut self) -> Result<Vec<u8>, String> {
        use std::io::{Seek, SeekFrom};
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("CleanupIdentityChanged: repositionner l'objet: {error}"))?;
        let mut bytes = Vec::new();
        self.file
            .read_to_end(&mut bytes)
            .map_err(|error| format!("CleanupIdentityChanged: relire l'objet vérifié: {error}"))?;
        Ok(bytes)
    }

    pub fn mark_delete(
        mut self,
        expected: &FileIdentity,
        expected_sha256: Option<&str>,
    ) -> Result<(), String> {
        self.verify(expected)?;
        if let Some(expected_sha256) = expected_sha256 {
            let mut hasher = sha2::Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let count = self
                    .file
                    .read(&mut buffer)
                    .map_err(|error| format!("CleanupIdentityChanged: relire l'objet: {error}"))?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
            }
            let actual = sha256_hex(&hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected_sha256) {
                return Err("CleanupIdentityChanged: empreinte de l'objet modifiée".into());
            }
        }
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        // SAFETY: `self.file` owns a live Windows handle; `disposition` is a
        // correctly initialized FILE_DISPOSITION_INFO that remains valid for
        // the duration of the call. The disposition is applied to this same
        // handle after verifying its file ID and rejecting reparse points.
        unsafe {
            SetFileInformationByHandle(
                HANDLE(self.file.as_raw_handle() as *mut core::ffi::c_void),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        }
        .map_err(|error| {
            format!("StorageUnavailable: disposition de suppression par handle: {error}")
        })
    }
}

fn identity_for_handle(file: &File, expect_directory: bool) -> Result<FileIdentity, String> {
    let mut id = FILE_ID_INFO::default();
    // SAFETY: `file` owns a valid handle, `id` is writable storage with the
    // exact Win32 structure layout, and the size matches the structure.
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle() as *mut core::ffi::c_void),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .map_err(|error| format!("StorageUnavailable: lire FileIdInfo: {error}"))?;

    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: `file` still owns the same live handle and `attributes` is
    // writable storage for the exact FILE_ATTRIBUTE_TAG_INFO structure.
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle() as *mut core::ffi::c_void),
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .map_err(|error| format!("StorageUnavailable: lire FileAttributeTagInfo: {error}"))?;
    let is_directory = attributes.FileAttributes & 0x10 != 0;
    if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err("CleanupUnsafePath: les points de réanalyse ne sont pas supprimables".into());
    }
    if is_directory != expect_directory {
        return Err("CleanupIdentityChanged: type fichier/dossier inattendu".into());
    }
    Ok(FileIdentity {
        volume_serial: id.VolumeSerialNumber,
        file_id: id.FileId.Identifier,
        is_directory,
    })
}

pub(crate) fn inspect(path: &Path, directory: bool) -> Result<FileIdentity, String> {
    let file = VerifiedFile::open(path, directory)?;
    Ok(file.identity)
}

pub(crate) fn delete_if_identity_matches(
    path: &Path,
    expected: &FileIdentity,
    expected_sha256: Option<&str>,
) -> Result<(), String> {
    let file = VerifiedFile::open(path, expected.is_directory)?;
    file.mark_delete(expected, expected_sha256)
}

pub(crate) fn open_and_hash_source(path: &Path) -> Result<VerifiedSource, String> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(path)
        .map_err(|error| format!("SourceChanged: ouvrir l'audio original: {error}"))?;
    let identity = identity_for_handle(&file, false)?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("SourceChanged: relire l'audio original: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(VerifiedSource {
        _file: file,
        identity,
        sha256: hasher.finalize().into(),
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    fn sandbox() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "rtc02-file-identity-{}-{:?}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn deletion_is_applied_to_the_open_identity() {
        let root = sandbox();
        let path = root.join("owned.txt");
        fs::write(&path, b"old").unwrap();
        let identity = inspect(&path, false).unwrap();
        delete_if_identity_matches(
            &path,
            &identity,
            Some(&sha256_hex(&sha2::Sha256::digest(b"old"))),
        )
        .unwrap();
        assert!(!path.exists());
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn mismatching_identity_is_not_deleted() {
        let root = sandbox();
        let path = root.join("owned.txt");
        fs::write(&path, b"keep").unwrap();
        let mut identity = inspect(&path, false).unwrap();
        identity.file_id[0] ^= 1;
        assert!(
            delete_if_identity_matches(
                &path,
                &identity,
                Some(&sha256_hex(&sha2::Sha256::digest(b"keep")))
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), b"keep");
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn verified_handle_blocks_path_substitution_until_disposition() {
        let root = sandbox();
        let path = root.join("owned.txt");
        let replacement = root.join("owned-replacement.txt");
        fs::write(&path, b"original").unwrap();
        let identity = inspect(&path, false).unwrap();
        let held = VerifiedFile::open(&path, false).unwrap();
        held.verify(&identity).unwrap();

        fs::write(&replacement, b"substitute").unwrap();
        assert!(
            fs::rename(&replacement, &path).is_err(),
            "open verified handle must deny rename/substitution"
        );
        held.verify(&identity).unwrap();
        drop(held);
        fs::remove_file(replacement).unwrap();
        delete_if_identity_matches(&path, &identity, None).unwrap();
        assert!(!path.exists());
        fs::remove_dir(root).unwrap();
    }
}
