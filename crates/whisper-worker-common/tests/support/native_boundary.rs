use std::{io::Write, process};

pub fn forbid(port: &'static str) -> ! {
    let _ = writeln!(io::stderr().lock(), "forbidden native test port reached: {port}");
    process::exit(78);
}
