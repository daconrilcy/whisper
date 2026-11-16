use sha2::{Digest, Sha256};
use std::io::{self, IsTerminal, Read, Write};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use whisper_core::cpu_compatibility::{
    CpuCapabilities, CpuCapabilityDiagnostic, CpuProcessRole, DIAGNOSTIC_PREFIX,
    MAX_DIAGNOSTIC_BYTES,
};

const DIAGNOSTIC_QUEUE_CAPACITY: usize = 8;
static DIAGNOSTIC_WRITER: OnceLock<Option<SyncSender<Vec<u8>>>> = OnceLock::new();
static DIAGNOSTIC_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn current() -> CpuCapabilities {
    read_current()
}

pub fn emit_cpu_worker_diagnostic() {
    emit_worker_diagnostic(CpuProcessRole::WorkerCpu);
}

pub fn emit_gpu_worker_diagnostic() {
    emit_worker_diagnostic(CpuProcessRole::WorkerGpu);
}

fn emit_worker_diagnostic(role: CpuProcessRole) {
    if DIAGNOSTIC_EMITTED.swap(true, Ordering::Relaxed) {
        return;
    }
    let registers = current();
    thread::spawn(move || {
        let Some(image_sha256) = current_image_sha256() else {
            return;
        };
        emit(role, std::process::id(), registers, Some(image_sha256));
    });
}

pub fn queue_worker_stderr_line(line: &str) {
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\n');
    if bytes.len() <= MAX_DIAGNOSTIC_BYTES {
        let _ = try_queue_diagnostic(diagnostic_writer(), bytes);
    }
}

pub fn diagnostic_line(
    role: CpuProcessRole,
    pid: u32,
    registers: CpuCapabilities,
) -> Option<Vec<u8>> {
    diagnostic_line_with_image(role, pid, registers, None)
}

fn diagnostic_line_with_image(
    role: CpuProcessRole,
    pid: u32,
    registers: CpuCapabilities,
    image_sha256: Option<String>,
) -> Option<Vec<u8>> {
    let mut record = CpuCapabilityDiagnostic::new(role, pid, registers);
    record.image_sha256 = image_sha256;
    let json = serde_json::to_vec(&record).ok()?;
    let mut line = Vec::with_capacity(DIAGNOSTIC_PREFIX.len() + 1 + json.len());
    line.extend_from_slice(DIAGNOSTIC_PREFIX.as_bytes());
    line.push(b' ');
    line.extend_from_slice(&json);
    (line.len() <= MAX_DIAGNOSTIC_BYTES).then_some(line)
}

fn emit(role: CpuProcessRole, pid: u32, registers: CpuCapabilities, image_sha256: Option<String>) {
    let Some(mut line) = diagnostic_line_with_image(role, pid, registers, image_sha256) else {
        return;
    };
    line.push(b'\n');
    let _ = try_queue_diagnostic(diagnostic_writer(), line);
}

fn current_image_sha256() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let count = file.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

fn diagnostic_writer() -> Option<&'static SyncSender<Vec<u8>>> {
    DIAGNOSTIC_WRITER
        .get_or_init(|| {
            if io::stderr().is_terminal() {
                return None;
            }
            let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(DIAGNOSTIC_QUEUE_CAPACITY);
            thread::spawn(move || {
                write_diagnostic_queue(receiver, |line| {
                    let _ = io::stderr().write_all(line);
                });
            });
            Some(sender)
        })
        .as_ref()
}

fn write_diagnostic_queue<F>(receiver: Receiver<Vec<u8>>, mut write: F)
where
    F: FnMut(&[u8]),
{
    while let Ok(line) = receiver.recv() {
        write(&line);
    }
}

fn try_queue_diagnostic(sender: Option<&SyncSender<Vec<u8>>>, line: Vec<u8>) -> bool {
    sender.is_some_and(|sender| sender.try_send(line).is_ok())
}

#[cfg(target_arch = "x86_64")]
fn read_current() -> CpuCapabilities {
    use std::arch::x86_64::__cpuid;

    let max_basic = __cpuid(0).eax;
    let max_extended = __cpuid(0x8000_0000).eax;
    let basic_leaf = (max_basic >= 1).then(|| __cpuid(1));
    let extended_leaf = (max_extended >= 0x8000_0001).then(|| __cpuid(0x8000_0001));
    let structured_leaf = (max_basic >= 7).then(|| __cpuid(7));

    CpuCapabilities {
        max_basic_leaf: Some(max_basic),
        max_extended_leaf: Some(max_extended),
        leaf_1_ecx: basic_leaf.map(|leaf| leaf.ecx),
        leaf_1_edx: basic_leaf.map(|leaf| leaf.edx),
        leaf_7_0_ebx: structured_leaf.map(|leaf| leaf.ebx),
        extended_leaf_1_ecx: extended_leaf.map(|leaf| leaf.ecx),
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn read_current() -> CpuCapabilities {
    CpuCapabilities::default()
}

#[cfg(test)]
mod tests;
