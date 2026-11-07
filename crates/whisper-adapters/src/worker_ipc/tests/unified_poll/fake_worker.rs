#![windows_subsystem = "windows"]

use std::io::{self, BufRead, Write};

fn string_field<'a>(line: &'a str, name: &str) -> &'a str {
    let prefix = format!("\"{name}\":\"");
    let value = line.split_once(&prefix).expect("string field").1;
    value.split_once('"').expect("string end").0
}

fn number_field<'a>(line: &'a str, name: &str) -> &'a str {
    let prefix = format!("\"{name}\":");
    let value = line.split_once(&prefix).expect("number field").1;
    value.split([',', '}']).next().expect("number end")
}

fn main() {
    let exe = std::env::current_exe().expect("current executable");
    let identity = std::fs::read_to_string(exe.with_extension("identity")).expect("identity");
    let mut wire = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(exe.with_extension("wire"))
        .expect("wire trace");
    while !exe.with_extension("release").is_file() {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let ready = format!(
        "{{\"version\":1,\"message\":{{\"type\":\"ready\",\"identity\":{identity},\"backend\":\"CPU\",\"native_version\":\"test\"}}}}"
    );
    writeln!(wire, "OUT {ready}").unwrap();
    println!("{ready}");
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines() {
        let line = line.expect("request frame");
        while !exe.with_extension("request-release").is_file() {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        writeln!(wire, "IN {line}").unwrap();
        wire.flush().unwrap();
        if line.contains("\"type\":\"shutdown\"") {
            break;
        }
        let id = number_field(&line, "request_id");
        let job = string_field(&line, "job_id");
        let generation = number_field(&line, "generation");
        let attempt = number_field(&line, "attempt");
        let sha = string_field(&line, "source_sha256");
        let response = format!(
            "{{\"version\":1,\"message\":{{\"type\":\"result\",\"request_id\":{id},\"job_id\":\"{job}\",\"generation\":{generation},\"attempt\":{attempt},\"identity\":{identity},\"source_sha256\":\"{sha}\",\"duration_samples\":48000,\"text\":\"fallback.\",\"words\":[{{\"text\":\"fallback.\",\"start_ms\":0,\"end_ms\":100}}]}}}}"
        );
        writeln!(wire, "OUT {response}").unwrap();
        wire.flush().unwrap();
        println!("{response}");
        io::stdout().flush().unwrap();
    }
}
