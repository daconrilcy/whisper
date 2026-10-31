use std::io::{self, BufRead, Write};

fn main() {
    let path = std::env::var_os("P1_OBSERVATION_PATH").expect("observation path");
    let mut observation = String::new();
    for arg in std::env::args().skip(1) {
        observation.push_str(&format!("arg={arg}\n"));
    }
    for name in [
        "PARAKEET_R6_TIMING_LOG",
        "PARAKEET_R6_CASE",
        "PARAKEET_R7_TIMING_ACTIVE",
        "PARAKEET_R7_EXEC_ROOT",
        "PARAKEET_R7_TIMING_LOG",
        "PARAKEET_L3C_EXEC_ROOT",
        "PARAKEET_L3D_EXEC_ROOT",
        "PARAKEET_L2_RUNTIME_ROOT",
        "PARAKEET_L3_ASR_LOG",
        "LOCALAPPDATA",
        "CUDA_PATH",
    ] {
        observation.push_str(&format!(
            "{name}={}\n",
            std::env::var(name).unwrap_or_else(|_| "<missing>".into())
        ));
    }
    std::fs::write(path, observation).unwrap();
    println!(r#"{{"version":1,"message":{{"type":"stopped"}}}}"#);
    io::stdout().flush().unwrap();
    let _ = io::stdin().lock().read_line(&mut String::new());
}
