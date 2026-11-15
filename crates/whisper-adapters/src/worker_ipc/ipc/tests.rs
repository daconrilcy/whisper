use super::*;
use crate::cpu_capabilities::{WorkerDiagnosticRelayState, drain_worker_stderr};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use whisper_core::cpu_compatibility::{
    CpuCapabilities, CpuCapabilityDiagnostic, CpuProcessRole, DIAGNOSTIC_PREFIX,
};

static TEST_IMAGE_ID: AtomicUsize = AtomicUsize::new(0);

fn diagnostic_line(pid: u32, image_sha256: [u8; 32]) -> Vec<u8> {
    let mut record =
        CpuCapabilityDiagnostic::new(CpuProcessRole::WorkerCpu, pid, CpuCapabilities::default());
    record.image_sha256 = Some(
        image_sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    );
    let json = serde_json::to_vec(&record).expect("CPU record serializes");
    [DIAGNOSTIC_PREFIX.as_bytes(), b" ", &json].concat()
}

fn image_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "whisper-transport-image-{}-{}",
        std::process::id(),
        TEST_IMAGE_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn transport_pins_t11_digest_before_spawn_and_rejects_same_path_substitution() {
    let root = image_root();
    let configured = root.join("configured/whisper-worker-cpu.exe");
    std::fs::create_dir_all(configured.parent().expect("configured parent exists"))
        .expect("configured parent can be created");
    let current = std::env::current_exe().expect("test PE path is available");
    crate::pe_fixture::copy_verified_pe(&current, &configured);
    let trusted_t11_digest = crate::archive::sha256_file(&configured)
        .expect("trusted build output digest is measurable");
    let pid = 81;
    let valid_line = diagnostic_line(pid, trusted_t11_digest);
    let valid_state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
    let mut valid_stderr = Cursor::new([valid_line.clone(), b"\n".to_vec()].concat());
    drain_worker_stderr(&mut valid_stderr, Arc::clone(&valid_state), |_| {});
    assert_eq!(
        valid_stderr.position() as usize,
        valid_stderr.get_ref().len()
    );
    let relayed = resolve_worker_diagnostic_identity(
        Some(trusted_t11_digest),
        Some(trusted_t11_digest),
        &configured,
        pid,
        &valid_state,
    )
    .expect("T11-pinned worker identity relays its first and only stderr line");
    assert_eq!(relayed, [valid_line, b"\n".to_vec()].concat());

    let distinct = std::fs::read_dir(current.parent().expect("test PE parent exists"))
        .expect("test PE directory can be read")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                && path != &current
                && crate::archive::sha256_file(path).is_ok_and(|hash| hash != trusted_t11_digest)
        })
        .expect("a discordant PE exists beside the test PE");
    crate::pe_fixture::copy_verified_pe(&distinct, &configured);
    let substituted_sha256 =
        crate::archive::sha256_file(&configured).expect("substituted image digest is measurable");
    assert_ne!(substituted_sha256, trusted_t11_digest);
    let substituted_line = diagnostic_line(pid, substituted_sha256);
    let substituted_state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
    let mut substituted_stderr =
        Cursor::new([substituted_line, b"\nordinary child output\n".to_vec()].concat());
    let total_bytes = substituted_stderr.get_ref().len();
    let mut forwarded = false;
    drain_worker_stderr(
        &mut substituted_stderr,
        Arc::clone(&substituted_state),
        |_| forwarded = true,
    );
    assert_eq!(substituted_stderr.position() as usize, total_bytes);
    assert!(
        resolve_worker_diagnostic_identity(
            Some(substituted_sha256),
            Some(trusted_t11_digest),
            &configured,
            pid,
            &substituted_state,
        )
        .is_none()
    );
    assert!(!forwarded, "same-path discordant PE is not relayed");
    std::fs::remove_dir_all(root).expect("temporary PE fixture root is removed");
}
