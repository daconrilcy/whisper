mod decoder;
mod diagnostics;
mod encoder;
mod engine_cache;
mod integrity;
#[allow(dead_code)]
mod ipc;
mod ipc_output;
#[cfg(feature = "l01-memory-qualification")]
mod memory_qualification;
mod native_engine;
mod qualification;
mod session;
mod startup;
mod worker;

fn main() {
    startup::run();
}
