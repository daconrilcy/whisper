use super::*;
use whisper_core::cpu_compatibility::{CpuCompatibilityLevel, CpuProcessRole};

#[test]
fn diagnostic_is_versioned_bounded_and_contains_process_local_registers() {
    let line = diagnostic_line(
        CpuProcessRole::WorkerCpu,
        37,
        CpuCapabilities {
            max_basic_leaf: Some(7),
            max_extended_leaf: Some(0x8000_0001),
            leaf_1_ecx: Some(0x0098_2201),
            leaf_1_edx: Some(0x0780_8101),
            leaf_7_0_ebx: Some(0xF000_0000),
            extended_leaf_1_ecx: Some(1),
        },
    )
    .expect("diagnostic fits the bound");

    assert!(line.len() <= MAX_DIAGNOSTIC_BYTES);
    assert!(line.starts_with(b"WHISPER_CPU_CAPABILITIES_V1 {"));
    let encoded = std::str::from_utf8(&line).expect("diagnostic is UTF-8");
    assert!(encoded.contains("\"role\":\"worker_cpu\""));
    assert!(encoded.contains("\"pid\":37"));
    assert!(encoded.contains("\"leaf_7_0_ebx\":4026531840"));
    assert!(encoded.contains("\"level\":\"x86_64_v2\""));
}

#[test]
fn gpu_worker_diagnostic_has_its_own_role_and_image_identity() {
    let line = diagnostic_line_with_image(
        CpuProcessRole::WorkerGpu,
        38,
        CpuCapabilities::default(),
        Some("ab".repeat(32)),
    )
    .expect("GPU worker diagnostic fits the bound");
    let encoded = std::str::from_utf8(&line).expect("diagnostic is UTF-8");
    assert!(encoded.contains("\"role\":\"worker_gpu\""));
    assert!(encoded.contains("\"pid\":38"));
    assert!(encoded.contains(&format!("\"image_sha256\":\"{}\"", "ab".repeat(32))));
}

#[test]
fn undetected_required_leaf_is_reported_as_indeterminate() {
    let diagnostic =
        CpuCapabilityDiagnostic::new(CpuProcessRole::WorkerCpu, 9, CpuCapabilities::default());
    assert_eq!(
        diagnostic.compatibility.level,
        CpuCompatibilityLevel::Indeterminate
    );
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
fn blocked_diagnostic_collector_does_not_hold_emission_progress() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let (writing_tx, writing_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        let mut block_first_write = true;
        write_diagnostic_queue(receiver, |_| {
            if block_first_write {
                block_first_write = false;
                let _ = writing_tx.send(());
                let _ = release_rx.recv();
            }
        });
    });
    assert!(try_queue_diagnostic(Some(&sender), b"first".to_vec()));
    writing_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("collector started writing");
    assert!(try_queue_diagnostic(Some(&sender), b"second".to_vec()));
    assert!(!try_queue_diagnostic(Some(&sender), b"third".to_vec()));
    release_tx.send(()).expect("collector can resume");
    drop(sender);
    writer.join().expect("diagnostic writer finishes");
}
