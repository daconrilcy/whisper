use std::{
    fs::{self, OpenOptions, TryLockError},
    sync::atomic::{AtomicU64, Ordering},
};

use whisper_adapters::archive::ArchiveStore;

#[test]
fn publication_lock_preserves_contents_and_exclusion_until_drop() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "publication-lock-contract-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join(".whisper-publications.lock");
    let sentinel = b"existing publication lock contents";
    fs::write(&path, sentinel).unwrap();

    let guard = ArchiveStore::lock_publications(&root).unwrap();
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(guard);
    contender.try_lock().unwrap();
    contender.unlock().unwrap();
    drop(contender);

    assert_eq!(fs::read(path).unwrap(), sentinel);
}
