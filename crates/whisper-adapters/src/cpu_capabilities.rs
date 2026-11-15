use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use whisper_core::cpu_compatibility::{
    CpuCapabilities, CpuCapabilityDiagnostic, CpuProcessRole, DIAGNOSTIC_PREFIX,
    MAX_DIAGNOSTIC_BYTES,
};

const STDERR_CHUNK_BYTES: usize = 256;
const DIAGNOSTIC_QUEUE_CAPACITY: usize = 8;
static DIAGNOSTIC_WRITER: OnceLock<Option<SyncSender<Vec<u8>>>> = OnceLock::new();

#[derive(Clone, Debug)]
pub(crate) struct WorkerDiagnosticIdentity {
    pid: u32,
    expected_executable: PathBuf,
    expected_sha256: [u8; 32],
}

#[derive(Default)]
pub(crate) struct WorkerDiagnosticRelayState {
    identity: Option<WorkerDiagnosticIdentity>,
    pending_line: Option<Vec<u8>>,
}

impl WorkerDiagnosticIdentity {
    pub(crate) fn launched(pid: u32, executable: &Path, expected_sha256: [u8; 32]) -> Option<Self> {
        Some(Self {
            pid,
            expected_executable: executable.to_path_buf(),
            expected_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn for_expected_hash(expected: &Path, expected_sha256: [u8; 32]) -> Option<Self> {
        Some(Self {
            pid: 0,
            expected_executable: expected.to_path_buf(),
            expected_sha256,
        })
    }

    pub(crate) fn with_pid(mut self, pid: u32) -> Self {
        self.pid = pid;
        self
    }

    fn is_cpu_worker(&self, record: &CpuCapabilityDiagnostic) -> bool {
        self.expected_executable
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("whisper-worker-cpu.exe"))
            && record.image_sha256.as_deref() == Some(hex_sha256(&self.expected_sha256).as_str())
    }
}

pub(crate) fn expected_cpu_worker_sha256() -> Option<[u8; 32]> {
    decode_sha256_hex(option_env!("WHISPER_EXPECTED_CPU_WORKER_SHA256")?)
}

fn decode_sha256_hex(encoded: &str) -> Option<[u8; 32]> {
    if encoded.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, pair) in encoded.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex_digit(pair[0])?;
        let low = decode_hex_digit(pair[1])?;
        digest[index] = (high << 4) | low;
    }
    Some(digest)
}

fn decode_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub fn emit_desktop_startup_diagnostic() {
    emit(CpuProcessRole::Desktop, std::process::id(), read_current());
}

pub(crate) fn validated_worker_diagnostic(
    line: &[u8],
    identity: Option<&WorkerDiagnosticIdentity>,
) -> Option<Vec<u8>> {
    let identity = identity?;
    if line.len() > MAX_DIAGNOSTIC_BYTES {
        return None;
    }
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let json = line
        .strip_prefix(DIAGNOSTIC_PREFIX.as_bytes())?
        .strip_prefix(b" ")?;
    let record: CpuCapabilityDiagnostic = serde_json::from_slice(json).ok()?;
    if !identity.is_cpu_worker(&record)
        || record.role != CpuProcessRole::WorkerCpu
        || record.pid != identity.pid
        || !record.is_consistent()
    {
        return None;
    }
    let mut forwarded = Vec::with_capacity(line.len() + 1);
    forwarded.extend_from_slice(line);
    forwarded.push(b'\n');
    Some(forwarded)
}

fn hex_sha256(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn drain_worker_stderr<R, F>(
    mut stderr: R,
    relay_state: Arc<Mutex<WorkerDiagnosticRelayState>>,
    mut forward: F,
) where
    R: Read,
    F: FnMut(&[u8]),
{
    let mut chunk = [0_u8; STDERR_CHUNK_BYTES];
    let mut line = Vec::with_capacity(MAX_DIAGNOSTIC_BYTES);
    let mut oversized = false;
    loop {
        let Ok(count) = stderr.read(&mut chunk) else {
            break;
        };
        if count == 0 {
            break;
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' {
                if !oversized {
                    if let Some(forwarded) = queue_or_validate_worker_line(&line, &relay_state) {
                        forward(&forwarded);
                    }
                }
                line.clear();
                oversized = false;
            } else if line.len() < MAX_DIAGNOSTIC_BYTES {
                line.push(*byte);
            } else {
                oversized = true;
            }
        }
    }
    if !oversized {
        if let Some(forwarded) = queue_or_validate_worker_line(&line, &relay_state) {
            forward(&forwarded);
        }
    }
}

pub(crate) fn install_worker_diagnostic_identity(
    relay_state: &Arc<Mutex<WorkerDiagnosticRelayState>>,
    identity: Option<WorkerDiagnosticIdentity>,
) -> Option<Vec<u8>> {
    let mut state = relay_state.lock().ok()?;
    if identity.is_none() {
        state.pending_line = None;
        return None;
    }
    state.identity = identity;
    let line = state.pending_line.take()?;
    validated_worker_diagnostic(&line, state.identity.as_ref())
}

fn queue_or_validate_worker_line(
    line: &[u8],
    relay_state: &Arc<Mutex<WorkerDiagnosticRelayState>>,
) -> Option<Vec<u8>> {
    if line.len() > MAX_DIAGNOSTIC_BYTES || !line.starts_with(DIAGNOSTIC_PREFIX.as_bytes()) {
        return None;
    }
    let mut state = relay_state.lock().ok()?;
    if let Some(identity) = state.identity.as_ref() {
        validated_worker_diagnostic(line, Some(identity))
    } else {
        if state.pending_line.is_none() {
            state.pending_line = Some(line.to_vec());
        }
        None
    }
}

fn emit(role: CpuProcessRole, pid: u32, registers: CpuCapabilities) {
    let record = CpuCapabilityDiagnostic::new(role, pid, registers);
    let Ok(json) = serde_json::to_vec(&record) else {
        return;
    };
    let mut line = Vec::with_capacity(DIAGNOSTIC_PREFIX.len() + 1 + json.len() + 1);
    line.extend_from_slice(DIAGNOSTIC_PREFIX.as_bytes());
    line.push(b' ');
    line.extend_from_slice(&json);
    if line.len() <= MAX_DIAGNOSTIC_BYTES {
        line.push(b'\n');
        let _ = try_queue_diagnostic(diagnostic_writer(), line);
    }
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

pub(crate) fn queue_forwarded_diagnostic(line: Vec<u8>) -> bool {
    try_queue_diagnostic(diagnostic_writer(), line)
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
