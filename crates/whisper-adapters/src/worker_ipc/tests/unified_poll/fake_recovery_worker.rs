#![windows_subsystem = "windows"]
use std::io::{self, BufRead, Write};
fn field<'a>(line: &'a str, name: &str, quoted: bool) -> &'a str {
    let prefix = if quoted {
        format!("\"{name}\":\"")
    } else {
        format!("\"{name}\":")
    };
    let value = line.split_once(&prefix).unwrap().1;
    if quoted {
        value.split_once('"').unwrap().0
    } else {
        value.split([',', '}']).next().unwrap()
    }
}
fn main() {
    let exe = std::env::current_exe().unwrap();
    let identity = std::fs::read_to_string(exe.with_extension("identity")).unwrap();
    let marker = exe.with_extension("first-exited");
    let first = !marker.is_file();
    let mut wire = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(exe.with_extension("wire"))
        .unwrap();
    let ready = format!(
        "{{\"version\":1,\"message\":{{\"type\":\"ready\",\"identity\":{identity},\"backend\":\"CPU\",\"native_version\":\"test\"}}}}"
    );
    writeln!(wire, "OUT {ready}").unwrap();
    wire.flush().unwrap();
    println!("{ready}");
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        writeln!(wire, "IN {line}").unwrap();
        wire.flush().unwrap();
        if line.contains("\"type\":\"shutdown\"") {
            break;
        }
        if first {
            std::fs::write(&marker, b"exited").unwrap();
            break;
        }
        let id = field(&line, "request_id", false);
        let job = field(&line, "job_id", true);
        let generation = field(&line, "generation", false);
        let attempt = field(&line, "attempt", false);
        let sha = field(&line, "source_sha256", true);
        let response = format!(
            "{{\"version\":1,\"message\":{{\"type\":\"result\",\"request_id\":{id},\"job_id\":\"{job}\",\"generation\":{generation},\"attempt\":{attempt},\"identity\":{identity},\"source_sha256\":\"{sha}\",\"duration_samples\":48000,\"text\":\"recovered.\",\"words\":[{{\"text\":\"recovered.\",\"start_ms\":0,\"end_ms\":100}}]}}}}"
        );
        writeln!(wire, "OUT {response}").unwrap();
        wire.flush().unwrap();
        println!("{response}");
        io::stdout().flush().unwrap();
    }
}
