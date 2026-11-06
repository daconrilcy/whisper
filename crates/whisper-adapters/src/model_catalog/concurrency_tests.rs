use super::{ModelFileHashCache, OpenModelFile};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, FileTimes},
    io::{self, Read},
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const WAIT: Duration = Duration::from_secs(10);
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "model-hash-concurrency-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn pause(entered: mpsc::SyncSender<()>, resume: mpsc::Receiver<()>) -> io::Result<()> {
    entered.send(()).map_err(io::Error::other)?;
    resume.recv_timeout(WAIT).map_err(io::Error::other)
}

#[test]
fn another_key_finishes_while_a_cold_read_is_suspended() {
    let root = TestRoot::new();
    let a = root.file("a.bin", b"alpha");
    let b = root.file("b.bin", b"bravo");
    let cache = Arc::new(ModelFileHashCache::default());
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let a_cache = Arc::clone(&cache);
    let first = thread::spawn(move || {
        a_cache.sha256_with(&a, |file, hasher| {
            pause(entered_tx, resume_rx)?;
            io::copy(file, hasher)
        })
    });
    entered_rx.recv_timeout(WAIT).unwrap();
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let second = thread::spawn(move || finished_tx.send(cache.sha256(&b)).unwrap());
    let other_result = finished_rx.recv_timeout(WAIT);
    // Release A before asserting, so a regression cannot strand its reader.
    resume_tx.send(()).unwrap();
    assert_eq!(other_result.unwrap().unwrap(), (5, digest(b"bravo"), true));
    assert_eq!(first.join().unwrap().unwrap(), (5, digest(b"alpha"), true));
    second.join().unwrap();
}

#[test]
fn concurrent_same_key_and_canonical_alias_share_one_read_and_warm_digest() {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"same identity");
    let alias = root.0.join(".").join("model.bin");
    let cache = Arc::new(ModelFileHashCache::default());
    let reads = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(12));
    let callers = (0..12)
        .map(|index| {
            let path = if index % 2 == 0 {
                path.clone()
            } else {
                alias.clone()
            };
            let cache = Arc::clone(&cache);
            let reads = Arc::clone(&reads);
            let start = Arc::clone(&start);
            thread::spawn(move || {
                start.wait();
                cache.sha256_with(&path, |file, hasher| {
                    reads.fetch_add(1, Ordering::Relaxed);
                    io::copy(file, hasher)
                })
            })
        })
        .collect::<Vec<_>>();
    let results = callers
        .into_iter()
        .map(|t| t.join().unwrap().unwrap())
        .collect::<Vec<_>>();
    assert!(
        results
            .iter()
            .all(|value| (value.0, value.1) == (13, digest(b"same identity")))
    );
    assert_eq!(results.iter().filter(|value| value.2).count(), 1);
    assert_eq!(reads.load(Ordering::Relaxed), 1);
    assert_eq!(cache.full_read_count(&alias).unwrap(), 1);
    let warm = cache.sha256_with(&path, |_, _| panic!("warm identity must not be reread"));
    assert_eq!(warm.unwrap(), (13, digest(b"same identity"), false));
}

#[test]
fn distinct_hardlink_paths_keep_their_existing_distinct_keys() {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let alias = root.0.join("hardlink.bin");
    fs::hard_link(&path, &alias).unwrap();
    let cache = ModelFileHashCache::default();
    let original_slot = cache.slot(&OpenModelFile::open(&path).unwrap()).unwrap();
    let alias_slot = cache.slot(&OpenModelFile::open(&alias).unwrap()).unwrap();
    assert!(!Arc::ptr_eq(&original_slot, &alias_slot));
    assert!(cache.sha256(&path).unwrap().2);
    assert!(cache.sha256(&alias).unwrap().2);
    assert_eq!(cache.full_read_count(&path).unwrap(), 1);
    assert_eq!(cache.full_read_count(&alias).unwrap(), 1);
}

fn change_file(path: &Path, replace: bool) {
    if replace {
        let modified = fs::metadata(path).unwrap().modified().unwrap();
        let replacement = path.with_extension("replacement");
        fs::write(&replacement, b"bravo").unwrap();
        File::options()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_times(FileTimes::new().set_modified(modified))
            .unwrap();
        fs::rename(path, path.with_extension("old")).unwrap();
        fs::rename(replacement, path).unwrap();
    } else {
        fs::write(path, b"changed size and contents").unwrap();
    }
}

fn stale_waiter_is_rejected(replace: bool) {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let cache = ModelFileHashCache::default();
    assert!(cache.sha256(&path).unwrap().2);
    let opened = OpenModelFile::open(&path).unwrap();
    let slot = cache.slot(&opened).unwrap();
    let held = slot.sha256.lock().unwrap();
    let waiting_slot = Arc::clone(&slot);
    let waiting_path = path.clone();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let waiter = thread::spawn(move || {
        entered_tx.send(()).unwrap();
        let result = waiting_slot.sha256(&waiting_path, opened, |_, _| {
            panic!("a stale waiter must be rejected before reading")
        });
        finished_tx.send(result).unwrap();
    });
    entered_rx.recv_timeout(WAIT).unwrap();
    change_file(&path, replace);
    drop(held);
    let error = finished_rx.recv_timeout(WAIT).unwrap().unwrap_err();
    assert!(error.to_string().contains("identity changed while waiting"));
    waiter.join().unwrap();
    assert!(slot.sha256.lock().unwrap().is_none());
    let current_bytes = fs::read(&path).unwrap();
    let current = cache.sha256(&path).unwrap();
    assert_eq!(current.1, digest(&current_bytes));
    assert!(current.2);
    assert_eq!(cache.full_read_count(&path).unwrap(), 1);
}

#[test]
fn mutation_while_waiting_rejects_a_previously_warm_digest() {
    stale_waiter_is_rejected(false);
}

#[test]
fn replacement_with_same_size_and_timestamp_while_waiting_rejects_warm_digest() {
    stale_waiter_is_rejected(true);
}

fn changed_during_read_is_rejected(replace: bool) {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let cache = Arc::new(ModelFileHashCache::default());
    let reader_cache = Arc::clone(&cache);
    let reader_path = path.clone();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        reader_cache.sha256_with(&reader_path, |file, hasher| {
            let mut prefix = [0; 2];
            file.read_exact(&mut prefix)?;
            hasher.update(prefix);
            pause(entered_tx, resume_rx)?;
            Ok(2 + io::copy(file, hasher)?)
        })
    });
    entered_rx.recv_timeout(WAIT).unwrap();
    change_file(&path, replace);
    resume_tx.send(()).unwrap();
    let error = reader.join().unwrap().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("identity changed while its SHA-256")
    );
    assert_eq!(cache.full_read_count(&path).unwrap(), 0);
    let current_bytes = fs::read(&path).unwrap();
    let current = cache.sha256(&path).unwrap();
    assert_eq!(current.1, digest(&current_bytes));
    assert!(current.2);
}

#[test]
fn mutation_during_read_is_rejected_without_publishing_a_digest() {
    changed_during_read_is_rejected(false);
}

#[test]
fn replacement_during_read_is_rejected_even_with_same_size_and_timestamp() {
    changed_during_read_is_rejected(true);
}

#[test]
fn short_read_is_not_published_and_next_request_retries() {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let cache = ModelFileHashCache::default();
    let error = cache.sha256_with(&path, |_, _| Ok(0)).unwrap_err();
    assert!(error.to_string().contains("identity changed"));
    assert_eq!(cache.full_read_count(&path).unwrap(), 0);
    assert_eq!(cache.sha256(&path).unwrap(), (5, digest(b"alpha"), true));
}

#[test]
fn reader_error_releases_same_key_waiter_which_can_retry() {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let cache = Arc::new(ModelFileHashCache::default());
    let failing_cache = Arc::clone(&cache);
    let failing_path = path.clone();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let failing = thread::spawn(move || {
        failing_cache.sha256_with(&failing_path, |_, _| {
            pause(entered_tx, resume_rx)?;
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected read failure",
            ))
        })
    });
    entered_rx.recv_timeout(WAIT).unwrap();
    let opened = OpenModelFile::open(&path).unwrap();
    let slot = cache.slot(&opened).unwrap();
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let retry_path = path.clone();
    let retry = thread::spawn(move || {
        finished_tx
            .send(slot.sha256(&retry_path, opened, |file, hasher| io::copy(file, hasher)))
            .unwrap();
    });
    resume_tx.send(()).unwrap();
    assert_eq!(
        failing.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        finished_rx.recv_timeout(WAIT).unwrap().unwrap(),
        (5, digest(b"alpha"), true)
    );
    retry.join().unwrap();
    assert_eq!(cache.full_read_count(&path).unwrap(), 1);
    assert!(!cache.sha256(&path).unwrap().2);
}

#[test]
fn panicked_reader_poison_is_explicit_and_another_key_still_progresses() {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let other = root.file("other.bin", b"bravo");
    let cache = ModelFileHashCache::default();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = cache.sha256_with(&path, |_, _| panic!("injected reader panic"));
    }));
    assert!(panic.is_err());
    for _ in 0..2 {
        assert!(
            cache
                .sha256(&path)
                .unwrap_err()
                .to_string()
                .contains("entry is poisoned")
        );
    }
    assert!(
        cache
            .full_read_count(&path)
            .unwrap_err()
            .to_string()
            .contains("entry is poisoned")
    );
    assert_eq!(cache.sha256(&other).unwrap(), (5, digest(b"bravo"), true));
}

#[test]
fn poisoned_global_index_is_an_explicit_error() {
    let root = TestRoot::new();
    let path = root.file("model.bin", b"alpha");
    let cache = ModelFileHashCache::default();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _guard = cache.entries.lock().unwrap();
        panic!("injected index panic");
    }));
    assert!(panic.is_err());
    assert!(
        cache
            .sha256(&path)
            .unwrap_err()
            .to_string()
            .contains("index is poisoned")
    );
    assert!(
        cache
            .full_read_count(&path)
            .unwrap_err()
            .to_string()
            .contains("index is poisoned")
    );
}
