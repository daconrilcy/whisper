use super::*;
use crate::cpu_capabilities::{WorkerDiagnosticRelayState, drain_worker_stderr};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use whisper_core::cpu_compatibility::{
    CpuCapabilities, CpuCapabilityDiagnostic, CpuProcessRole, DIAGNOSTIC_PREFIX,
};

static TEST_IMAGE_ID: AtomicUsize = AtomicUsize::new(0);

fn diagnostic_line(role: CpuProcessRole, pid: u32, image_sha256: [u8; 32]) -> Vec<u8> {
    let mut record = CpuCapabilityDiagnostic::new(role, pid, CpuCapabilities::default());
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
    let valid_line = diagnostic_line(CpuProcessRole::WorkerCpu, pid, trusted_t11_digest);
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
    let substituted_line = diagnostic_line(CpuProcessRole::WorkerCpu, pid, substituted_sha256);
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

#[test]
fn gpu_executable_diagnostic_is_relayed_only_for_its_pid_role_and_image() {
    let image_sha256 = [7_u8; 32];
    let line = diagnostic_line(CpuProcessRole::WorkerGpu, 92, image_sha256);
    let input = [line.clone(), b"\nordinary worker output\n".to_vec()].concat();
    let cpu_state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
    assert!(
        resolve_worker_diagnostic_identity(
            Some(image_sha256),
            Some(image_sha256),
            std::path::Path::new("whisper-worker-cpu.exe"),
            92,
            &cpu_state,
        )
        .is_none()
    );
    let mut cpu_stderr = Cursor::new(input.clone());
    let mut cpu_forwarded = Vec::new();
    drain_worker_stderr(&mut cpu_stderr, cpu_state, |forwarded| {
        cpu_forwarded.push(forwarded.to_vec())
    });
    assert_eq!(cpu_stderr.position() as usize, cpu_stderr.get_ref().len());
    assert!(
        cpu_forwarded.is_empty(),
        "GPU role is not relayed for the CPU PE"
    );

    let gpu_state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
    let mut gpu_stderr = Cursor::new(input);
    let mut gpu_forwarded = Vec::new();
    drain_worker_stderr(&mut gpu_stderr, Arc::clone(&gpu_state), |forwarded| {
        gpu_forwarded.push(forwarded.to_vec())
    });

    assert_eq!(gpu_stderr.position() as usize, gpu_stderr.get_ref().len());
    assert!(
        gpu_forwarded.is_empty(),
        "unidentified diagnostic remains pending"
    );

    let verified_gpu_line = resolve_worker_diagnostic_identity(
        Some(image_sha256),
        None,
        std::path::Path::new("whisper-worker-gpu.exe"),
        92,
        &gpu_state,
    )
    .expect("observed GPU PE identity validates the pending matching record");
    assert_eq!(verified_gpu_line, [line, b"\n".to_vec()].concat());

    for (diagnostic_pid, diagnostic_sha) in [(93, image_sha256), (92, [8_u8; 32])] {
        let mismatched_line =
            diagnostic_line(CpuProcessRole::WorkerGpu, diagnostic_pid, diagnostic_sha);
        let mismatched_state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
        let mut mismatched_stderr = Cursor::new([mismatched_line, b"\n".to_vec()].concat());
        drain_worker_stderr(
            &mut mismatched_stderr,
            Arc::clone(&mismatched_state),
            |_| {},
        );
        assert!(
            resolve_worker_diagnostic_identity(
                Some(image_sha256),
                None,
                std::path::Path::new("whisper-worker-gpu.exe"),
                92,
                &mismatched_state,
            )
            .is_none()
        );
    }
}

#[test]
fn gpu_diagnostic_without_an_opened_image_identity_is_not_relayed() {
    let image_sha256 = [4_u8; 32];
    let line = diagnostic_line(CpuProcessRole::WorkerGpu, 92, image_sha256);
    let state = Arc::new(Mutex::new(WorkerDiagnosticRelayState::default()));
    let mut stderr = Cursor::new([line, b"\n".to_vec()].concat());
    drain_worker_stderr(&mut stderr, Arc::clone(&state), |_| {});

    assert!(
        resolve_worker_diagnostic_identity(
            None,
            None,
            std::path::Path::new("whisper-worker-gpu.exe"),
            92,
            &state,
        )
        .is_none()
    );
}

#[cfg(windows)]
#[test]
fn pinned_worker_image_blocks_replacement_and_allows_process_launch() {
    let root = image_root();
    std::fs::create_dir_all(&root).expect("fixture root can be created");
    let current = std::env::current_exe().expect("test PE path is available");
    let image = root.join("whisper-worker-gpu.exe");
    crate::pe_fixture::copy_verified_pe(&current, &image);
    let image = image.canonicalize().expect("worker fixture canonicalizes");
    let pin = open_gpu_worker_image(&image).expect("worker image opens with a read-only share");

    let write_open = std::fs::OpenOptions::new().write(true).open(&image);
    assert!(write_open.is_err(), "pinned image rejects write access");
    let replacement = image.with_extension("replacement.exe");
    assert!(
        std::fs::rename(&image, &replacement).is_err(),
        "pinned image rejects rename/delete access"
    );

    let child = Command::new(&image)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Windows can launch a read-only-pinned image");
    assert!(
        child
            .wait_with_output()
            .expect("test process exits")
            .status
            .success()
    );
    drop(pin);
    std::fs::remove_dir_all(root).expect("fixture root can be removed after pin release");
}
