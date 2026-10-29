mod decoder;
mod diagnostics;
mod encoder;
mod engine_cache;
mod gpu_attestation;
mod gpu_commands;
mod gpu_qualification;
mod integrity;
mod ipc;
#[cfg(feature = "l01-memory-qualification")]
mod memory_qualification;
mod native_engine;
mod native_ffi;
mod qualification;
mod session;
mod startup;
mod worker;
fn main() {
    startup::run();
}
