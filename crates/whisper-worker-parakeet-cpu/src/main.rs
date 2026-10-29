mod args;
mod decode_only;
mod decoder;
mod gpu_identity;
mod integrity;
mod ipc;
mod native_engine;
mod native_ffi;
#[cfg(feature = "parakeet-qualification-reference")]
mod reference_decoder;
mod runtime_paths;
mod session;
mod startup;
mod timing;
mod transcription;
use timing::record_r7_timing;

fn main() {
    if let Err(error) = startup::run() {
        eprintln!("parakeet-worker fatal: {error}");
        std::process::exit(2);
    }
}
