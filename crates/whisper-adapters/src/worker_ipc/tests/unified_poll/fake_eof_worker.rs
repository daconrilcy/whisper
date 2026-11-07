#![windows_subsystem = "windows"]
use std::io::{self, BufRead, Write};
fn main() {
    let exe = std::env::current_exe().unwrap();
    let identity = std::fs::read_to_string(exe.with_extension("identity")).unwrap();
    let mut wire = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(exe.with_extension("wire"))
        .unwrap();
    let ready = format!(
        "{{\"version\":1,\"message\":{{\"type\":\"ready\",\"identity\":{identity},\"backend\":\"CPU\",\"native_version\":\"test\"}}}}"
    );
    writeln!(wire, "OUT {ready}").unwrap();
    println!("{ready}");
    io::stdout().flush().unwrap();
    if let Some(line) = io::stdin().lock().lines().next() {
        writeln!(wire, "IN {}", line.unwrap()).unwrap();
        wire.flush().unwrap();
    }
}
