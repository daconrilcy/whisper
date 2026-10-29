use std::{
    io::{Read, Write},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Output, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const CONTROLLED_TIMEOUT: Duration = Duration::from_secs(300);
pub const NATIVE_TIMEOUT: Duration = Duration::from_secs(600);

pub struct BoundedChild {
    child: Arc<Mutex<Child>>,
    deadline: Instant,
    expired: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    watchdog: Option<JoinHandle<()>>,
}

impl BoundedChild {
    pub fn spawn(command: &mut Command, timeout: Duration) -> Self {
        let child = Arc::new(Mutex::new(command.spawn().expect("spawn owned child")));
        let deadline = Instant::now() + timeout;
        let expired = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let watch_child = Arc::clone(&child);
        let watch_expired = Arc::clone(&expired);
        let watch_done = Arc::clone(&done);
        let watchdog = thread::spawn(move || {
            while !watch_done.load(Ordering::Acquire) {
                if Instant::now() >= deadline {
                    watch_expired.store(true, Ordering::Release);
                    let _ = watch_child.lock().unwrap().kill();
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        Self {
            child,
            deadline,
            expired,
            done,
            watchdog: Some(watchdog),
        }
    }

    pub fn id(&self) -> u32 {
        self.child.lock().unwrap().id()
    }
    pub fn stdin(&self) -> ChildStdin {
        self.child.lock().unwrap().stdin.take().unwrap()
    }
    pub fn stdout(&self) -> ChildStdout {
        self.child.lock().unwrap().stdout.take().unwrap()
    }
    pub fn stderr(&self) -> ChildStderr {
        self.child.lock().unwrap().stderr.take().unwrap()
    }
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
    pub fn timed_out(&self) -> bool {
        self.expired.load(Ordering::Acquire) || self.remaining().is_zero()
    }

    pub fn wait(&self) -> Result<ExitStatus, String> {
        loop {
            if let Some(status) = self
                .child
                .lock()
                .unwrap()
                .try_wait()
                .map_err(|e| e.to_string())?
            {
                if self.timed_out() {
                    return Err("owned child exceeded deadline and was stopped".into());
                }
                return Ok(status);
            }
            if self.timed_out() {
                let _ = self.child.lock().unwrap().kill();
                let reap_deadline = Instant::now() + Duration::from_secs(2);
                while Instant::now() < reap_deadline {
                    if self
                        .child
                        .lock()
                        .unwrap()
                        .try_wait()
                        .ok()
                        .flatten()
                        .is_some()
                    {
                        break;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                return Err("owned child exceeded deadline and was stopped".into());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for BoundedChild {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        let _ = self.child.lock().unwrap().kill();
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            if self
                .child
                .lock()
                .unwrap()
                .try_wait()
                .ok()
                .flatten()
                .is_some()
            {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
    }
}

pub fn finish_reader<T>(reader: JoinHandle<T>, child: &BoundedChild) -> T {
    while !reader.is_finished() {
        assert!(
            !child.timed_out(),
            "output reader exceeded the owned child deadline"
        );
        thread::sleep(Duration::from_millis(20));
    }
    reader.join().ok().expect("output reader panicked")
}

pub fn output(command: &mut Command, input: Option<Vec<u8>>, timeout: Duration) -> Output {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = BoundedChild::spawn(command, timeout);
    let stdout = child.stdout();
    let stderr = child.stderr();
    let (tx, rx) = mpsc::channel();
    for (kind, mut stream) in [
        (0, Box::new(stdout) as Box<dyn Read + Send>),
        (1, Box::new(stderr) as Box<dyn Read + Send>),
    ] {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stream.read_to_end(&mut bytes);
            let _ = tx.send((kind, bytes));
        });
    }
    drop(tx);
    let write_result = if let Some(input) = input {
        let mut stdin = child.stdin();
        stdin.write_all(&input)
    } else {
        Ok(())
    };
    let result = child.wait();
    let mut captured = [Vec::new(), Vec::new()];
    let mut received = 0;
    for _ in 0..2 {
        let remaining = if result.is_ok() {
            child.remaining()
        } else {
            Duration::from_secs(2)
        };
        if let Ok((kind, bytes)) = rx.recv_timeout(remaining) {
            captured[kind] = bytes;
            received += 1;
        }
    }
    let status = result.unwrap_or_else(|error| {
        panic!(
            "{error}; stdout={:?}; stderr={:?}",
            String::from_utf8_lossy(&captured[0]),
            String::from_utf8_lossy(&captured[1])
        )
    });
    assert_eq!(
        received,
        2,
        "output capture timed out; stdout={:?}; stderr={:?}",
        String::from_utf8_lossy(&captured[0]),
        String::from_utf8_lossy(&captured[1])
    );
    write_result.expect("write owned child input");
    Output {
        status,
        stdout: std::mem::take(&mut captured[0]),
        stderr: std::mem::take(&mut captured[1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_child_fixture() {
        if std::env::var_os("WHISPER_TEST_DEADLINE_FIXTURE").is_some() {
            println!("fixture-ready");
            std::io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(10));
        }
    }

    #[test]
    fn deadline_stops_child_after_expected_output() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("deadline_child_fixture")
            .arg("--nocapture")
            .env("WHISPER_TEST_DEADLINE_FIXTURE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = BoundedChild::spawn(&mut command, Duration::from_secs(3));
        let (tx, rx) = mpsc::channel();
        let mut stdout = std::io::BufReader::new(child.stdout());
        thread::spawn(move || {
            let mut line = String::new();
            use std::io::BufRead;
            while stdout.read_line(&mut line).unwrap_or(0) > 0 {
                if line.contains("fixture-ready") {
                    let _ = tx.send(());
                    return;
                }
                line.clear();
            }
        });
        rx.recv_timeout(Duration::from_secs(2))
            .expect("fixture did not produce its expected output");
        assert!(
            child.wait().is_err(),
            "living child must be stopped at deadline"
        );
        assert!(
            child.child.lock().unwrap().try_wait().unwrap().is_some(),
            "stopped child must be reaped"
        );
    }

    #[test]
    fn timed_out_output_retains_the_captured_stdout() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("deadline_child_fixture")
            .arg("--nocapture")
            .env("WHISPER_TEST_DEADLINE_FIXTURE", "1");
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            output(&mut command, None, Duration::from_secs(3));
        }))
        .expect_err("fixture must exceed the deadline");
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(
            message.contains("fixture-ready"),
            "captured output missing: {message}"
        );
    }
}
