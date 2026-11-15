use super::*;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use whisper_core::cpu_compatibility::CpuCompatibilityLevel;

static TEST_IMAGE_ID: AtomicUsize = AtomicUsize::new(0);

fn worker_line(pid: u32) -> Vec<u8> {
    let current = std::env::current_exe().expect("test executable path is available");
    let hash = crate::archive::sha256_file(&current).expect("test image hash succeeds");
    worker_line_with_hash(pid, hash)
}

fn worker_line_with_hash(pid: u32, hash: [u8; 32]) -> Vec<u8> {
    let mut record = CpuCapabilityDiagnostic::new(
        CpuProcessRole::WorkerCpu,
        pid,
        CpuCapabilities {
            max_basic_leaf: Some(7),
            max_extended_leaf: Some(0x8000_0001),
            leaf_1_ecx: Some(0x0098_2201),
            leaf_1_edx: Some(0x0780_8101),
            leaf_7_0_ebx: Some(0),
            extended_leaf_1_ecx: Some(1),
        },
    );
    record.image_sha256 = Some(hash.iter().map(|byte| format!("{byte:02x}")).collect());
    serde_json::to_vec(&record)
        .map(|json| [DIAGNOSTIC_PREFIX.as_bytes(), b" ", &json].concat())
        .expect("diagnostic serialization succeeds")
}

fn cpu_identity(pid: u32) -> WorkerDiagnosticIdentity {
    let current = std::env::current_exe().expect("test executable path is available");
    let expected_sha256 =
        crate::archive::sha256_file(&current).expect("expected CPU image hash is measurable");
    WorkerDiagnosticIdentity::for_expected_hash(
        Path::new("whisper-worker-cpu.exe"),
        expected_sha256,
    )
    .expect("launch image identity is measurable")
    .with_pid(pid)
}

#[test]
fn decodes_the_t11_worker_digest_as_exact_sha256_bytes() {
    assert_eq!(decode_sha256_hex(&"a5".repeat(32)), Some([0xa5; 32]));
    assert!(decode_sha256_hex("a5").is_none());
    assert!(decode_sha256_hex(&"gg".repeat(32)).is_none());
}

fn drain_with_identity<R, F>(stderr: R, identity: Option<WorkerDiagnosticIdentity>, forward: F)
where
    R: Read,
    F: FnMut(&[u8]),
{
    let state = Arc::new(Mutex::new(WorkerDiagnosticRelayState {
        identity,
        pending_line: None,
    }));
    drain_worker_stderr(stderr, state, forward);
}

fn test_image_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "whisper-cpu-image-{}-{}",
        std::process::id(),
        TEST_IMAGE_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn forwards_only_allowlisted_pid_attributed_cpu_diagnostics() {
    let valid = worker_line(81);
    let identity = cpu_identity(81);
    let forwarded =
        validated_worker_diagnostic(&valid, Some(&identity)).expect("matching worker line");
    assert_eq!(forwarded.strip_suffix(b"\n"), Some(valid.as_slice()));
    assert!(validated_worker_diagnostic(&valid, Some(&cpu_identity(82))).is_none());

    let desktop = serde_json::to_vec(&CpuCapabilityDiagnostic::new(
        CpuProcessRole::Desktop,
        81,
        CpuCapabilities::default(),
    ))
    .map(|json| [DIAGNOSTIC_PREFIX.as_bytes(), b" ", &json].concat())
    .expect("diagnostic serialization succeeds");
    assert!(validated_worker_diagnostic(&desktop, Some(&identity)).is_none());
    assert!(validated_worker_diagnostic(b"ordinary native diagnostic", Some(&identity)).is_none());

    let mut other_worker = valid.clone();
    let role = b"worker_cpu";
    let role_start = other_worker
        .windows(role.len())
        .position(|window| window == role)
        .expect("worker role is present");
    other_worker[role_start..role_start + role.len()].copy_from_slice(b"worker_gpu");
    assert!(validated_worker_diagnostic(&other_worker, Some(&identity)).is_none());
}

#[test]
fn rejects_inconsistent_oversized_and_malformed_diagnostics() {
    let valid = worker_line(81);
    let mut inconsistent: CpuCapabilityDiagnostic =
        serde_json::from_slice(&valid[DIAGNOSTIC_PREFIX.len() + 1..])
            .expect("serialized worker line parses");
    inconsistent.compatibility.level = CpuCompatibilityLevel::BelowX86_64V2;
    let inconsistent = serde_json::to_vec(&inconsistent)
        .map(|json| [DIAGNOSTIC_PREFIX.as_bytes(), b" ", &json].concat())
        .expect("diagnostic serialization succeeds");
    let identity = cpu_identity(81);
    assert!(validated_worker_diagnostic(&inconsistent, Some(&identity)).is_none());
    assert!(
        validated_worker_diagnostic(&vec![b'x'; MAX_DIAGNOSTIC_BYTES + 1], Some(&identity))
            .is_none()
    );
    assert!(validated_worker_diagnostic(b"{malformed JSON", Some(&identity)).is_none());
}

#[test]
fn stderr_is_fully_drained_while_unlisted_or_long_lines_are_discarded() {
    let mut input = b"native log\n".to_vec();
    input.extend(std::iter::repeat_n(b'x', MAX_DIAGNOSTIC_BYTES + 10));
    input.extend_from_slice(b"\n");
    input.extend_from_slice(&worker_line(81));
    input.push(b'\n');
    let mut output = Vec::new();

    drain_with_identity(Cursor::new(input), Some(cpu_identity(81)), |line| {
        output.extend_from_slice(line)
    });

    assert_eq!(output, [worker_line(81), b"\n".to_vec()].concat());
}

#[test]
fn transport_relays_a_valid_cpu_identity_as_its_only_stderr_line() {
    let line = worker_line(81);
    let expected = cpu_identity(81);
    let mut forwarded = Vec::new();
    drain_with_identity(
        Cursor::new([line.clone(), b"\n".to_vec()].concat()),
        Some(expected),
        |output| forwarded.extend_from_slice(output),
    );
    assert_eq!(forwarded, [line, b"\n".to_vec()].concat());
}

#[test]
fn stderr_drain_finishes_while_identity_hash_is_pending_and_relays_first_line_on_resolution() {
    let line = worker_line(81);
    let state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
    let input = [line.clone(), b"\nordinary output\n".to_vec()].concat();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (resolved_tx, resolved_rx) = std::sync::mpsc::channel();
    let resolver_state = Arc::clone(&state);
    let resolver = std::thread::spawn(move || {
        release_rx
            .recv()
            .expect("identity verification is released");
        let forwarded = install_worker_diagnostic_identity(&resolver_state, Some(cpu_identity(81)));
        let _ = resolved_tx.send(forwarded);
    });
    let (drained_tx, drained_rx) = std::sync::mpsc::channel();
    let drain_state = Arc::clone(&state);
    let drain = std::thread::spawn(move || {
        let mut stderr = Cursor::new(input);
        drain_worker_stderr(&mut stderr, drain_state, |_| {});
        let _ = drained_tx.send((stderr.position() as usize, stderr.get_ref().len()));
    });
    assert_eq!(
        drained_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("stderr drain does not wait for image verification"),
        (
            line.len() + b"\nordinary output\n".len(),
            line.len() + b"\nordinary output\n".len()
        )
    );
    release_tx.send(()).expect("identity thread can continue");
    assert_eq!(
        resolved_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("queued first diagnostic is resolved"),
        Some([line, b"\n".to_vec()].concat())
    );
    resolver.join().expect("identity resolver completes");
    drain.join().expect("stderr drainer completes");
}

#[test]
fn unavailable_or_full_diagnostic_queue_drops_without_waiting() {
    assert!(!try_queue_diagnostic(None, b"line".to_vec()));
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    assert!(try_queue_diagnostic(Some(&sender), b"first".to_vec()));
    assert!(!try_queue_diagnostic(Some(&sender), b"second".to_vec()));
    assert_eq!(receiver.try_recv().as_deref(), Ok(b"first".as_slice()));
}

#[test]
fn full_forwarding_queue_does_not_stop_stderr_drain() {
    let input = [worker_line(81), worker_line(81)].concat();
    let mut stderr = Cursor::new(input);
    let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
    drain_with_identity(&mut stderr, Some(cpu_identity(81)), |line| {
        let _ = try_queue_diagnostic(Some(&sender), line.to_vec());
    });
    assert_eq!(stderr.position() as usize, stderr.get_ref().len());
}

#[test]
fn rejects_a_different_pe_copied_under_the_expected_basename() {
    let root = test_image_root();
    let expected = root.join("configured/whisper-worker-cpu.exe");
    std::fs::create_dir_all(expected.parent().expect("image parent exists"))
        .expect("image parent can be created");
    let current = std::env::current_exe().expect("test executable path is available");
    let current_hash = crate::archive::sha256_file(&current).expect("test PE hash succeeds");
    let distinct_pe = std::fs::read_dir(current.parent().expect("test executable parent exists"))
        .expect("test executable directory can be read")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                && path != &current
                && crate::archive::sha256_file(path).is_ok_and(|hash| hash != current_hash)
        })
        .expect("a distinct test PE exists beside the current test executable");
    crate::pe_fixture::copy_verified_pe(&current, &expected);
    let trusted_hash = crate::archive::sha256_file(&expected).expect("trusted PE hash succeeds");
    let expected_identity = WorkerDiagnosticIdentity::for_expected_hash(&expected, trusted_hash)
        .expect("trusted image identity is measurable")
        .with_pid(81);
    let line = worker_line(81);
    assert!(validated_worker_diagnostic(&line, Some(&expected_identity)).is_some());

    crate::pe_fixture::copy_verified_pe(&distinct_pe, &expected);
    let distinct_hash =
        crate::archive::sha256_file(&distinct_pe).expect("discordant PE hash succeeds");
    let substituted_line = worker_line_with_hash(81, distinct_hash);

    assert!(validated_worker_diagnostic(&substituted_line, Some(&expected_identity)).is_none());
    let mut stderr = Cursor::new(
        [
            b"ordinary output\n".as_slice(),
            substituted_line.as_slice(),
            b"\n".as_slice(),
        ]
        .concat(),
    );
    let mut forwarded = false;
    drain_with_identity(&mut stderr, Some(expected_identity), |_| forwarded = true);
    assert!(!forwarded, "substituted PE diagnostic is rejected");
    assert_eq!(stderr.position() as usize, stderr.get_ref().len());
    std::fs::remove_dir_all(root).expect("temporary images are removed");
}

#[test]
fn blocked_diagnostic_collector_does_not_hold_emission_or_stderr_drain() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let (writing_tx, writing_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        write_diagnostic_queue(receiver, |_| {
            let _ = writing_tx.send(());
            let _ = release_rx.recv();
        });
    });
    assert!(try_queue_diagnostic(Some(&sender), b"first".to_vec()));
    writing_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("collector started writing");
    let mut stderr = Cursor::new([worker_line(81), worker_line(81)].concat());
    drain_with_identity(&mut stderr, Some(cpu_identity(81)), |line| {
        let _ = try_queue_diagnostic(Some(&sender), line.to_vec());
    });
    assert_eq!(stderr.position() as usize, stderr.get_ref().len());
    release_tx.send(()).expect("collector can resume");
    drop(sender);
    writer.join().expect("diagnostic writer finishes");
}
