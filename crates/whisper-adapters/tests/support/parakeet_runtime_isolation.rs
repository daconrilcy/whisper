use crate::parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const HISTORICAL: [&str; 7] = [
    "PARAKEET_R6_TIMING_LOG",
    "PARAKEET_R6_CASE",
    "PARAKEET_R7_TIMING_ACTIVE",
    "PARAKEET_R7_EXEC_ROOT",
    "PARAKEET_R7_TIMING_LOG",
    "PARAKEET_L3C_EXEC_ROOT",
    "PARAKEET_L3D_EXEC_ROOT",
];
const CHILD_ROOT: &str = "WHISPER_P1_ISOLATION_ROOT";

fn run_matrix(spawn: impl Fn(&ParakeetWorkerPaths, &Path) -> Result<ParakeetProcess, String>) {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let backend = std::env::var("WHISPER_P1_BACKEND").unwrap();
        let personal = root.join("Whisper");
        let paths = ParakeetWorkerPaths {
            executable: personal.join(format!("worker/whisper-worker-parakeet-{backend}.exe")),
            native_dir: personal.join(format!("native/{backend}")),
            model: personal.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
            runtime_root: personal.clone(),
        };
        let session = root.join("session");
        let mut process = spawn(&paths, &session).expect("personal worker must start");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            match process.poll().expect("worker IPC") {
                Some(whisper_core::parakeet_ipc::Event::Stopped) => break,
                Some(other) => panic!("unexpected fixture event: {other:?}"),
                None if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                None => panic!("fixture worker did not send IPC event"),
            }
        }
        let observation = std::fs::read_to_string(root.join("observation.txt")).unwrap();
        for (index, key) in HISTORICAL.into_iter().enumerate() {
            let expected = if backend == "historical" {
                match index {
                    5 => "p1-invalid-l3c",
                    6 => "p1-invalid-l3d",
                    _ => "p1-invalid-campaign",
                }
            } else {
                "<missing>"
            };
            assert!(
                observation.contains(&format!("{key}={expected}\n")),
                "{key}: {observation}"
            );
        }
        for (key, value) in [
            ("LOCALAPPDATA", root.as_path()),
            ("PARAKEET_L2_RUNTIME_ROOT", personal.as_path()),
        ] {
            assert!(
                observation.contains(&format!("{key}={}\n", value.display())),
                "{observation}"
            );
        }
        assert!(observation.contains("CUDA_PATH=p1-cuda-sentinel\n"));
        for argument in [
            "--model".to_owned(),
            paths.model.display().to_string(),
            "--model-sha256".to_owned(),
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256
            .to_owned(),
            "--native-dir".to_owned(),
            paths.native_dir.display().to_string(),
        ] {
            assert!(
                observation
                    .lines()
                    .any(|line| line == format!("arg={argument}")),
                "{observation}"
            );
        }
        if backend == "gpu" {
            let log = observation
                .lines()
                .find_map(|line| line.strip_prefix("PARAKEET_L3_ASR_LOG="))
                .unwrap();
            let log = PathBuf::from(log);
            assert!(log.is_file(), "GPU log absent: {}", log.display());
            assert!(
                log.canonicalize()
                    .unwrap()
                    .starts_with(personal.join("settings/logs").canonicalize().unwrap())
            );
        }
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("parakeet-p1-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    let fake_exe =
        PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("isolated Cargo target")).join(
            format!("parakeet-p1-fake-{}-{stamp}.exe", std::process::id()),
        );
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/parakeet_fake_worker.rs");
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("--edition=2024")
        .arg("-Cdebuginfo=0")
        .arg(&source)
        .arg("-o")
        .arg(&fake_exe)
        .output()
        .expect("compile fixture worker");
    assert!(
        compiled.status.success(),
        "fixture compile failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let test_name = thread::current().name().expect("test name").to_owned();
    for (backend, pollution) in [
        ("cpu", "clean"),
        ("cpu", "l3c"),
        ("cpu", "l3d"),
        ("cpu", "both"),
        ("gpu", "clean"),
        ("gpu", "l3c"),
        ("gpu", "l3d"),
        ("gpu", "both"),
        ("historical", "both"),
    ] {
        let root = base.join(format!("{backend}-{pollution}"));
        let personal = root.join("Whisper");
        let worker = personal.join(format!("worker/whisper-worker-parakeet-{backend}.exe"));
        std::fs::create_dir_all(worker.parent().unwrap()).unwrap();
        std::fs::create_dir_all(personal.join(format!("native/{backend}"))).unwrap();
        std::fs::create_dir_all(personal.join("models")).unwrap();
        std::fs::create_dir_all(personal.join("settings")).unwrap();
        std::fs::create_dir_all(root.join("session")).unwrap();
        std::fs::copy(&fake_exe, &worker).unwrap();
        std::fs::write(
            personal.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
            b"fixture",
        )
        .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .arg("--exact")
            .arg(&test_name)
            .arg("--nocapture")
            .env(CHILD_ROOT, &root)
            .env("WHISPER_P1_BACKEND", backend)
            .env("P1_OBSERVATION_PATH", root.join("observation.txt"))
            .env("LOCALAPPDATA", &root)
            .env("CUDA_PATH", "p1-cuda-sentinel")
            .env("PARAKEET_L2_RUNTIME_ROOT", "p1-invalid-parent-root")
            .env("PARAKEET_L3_ASR_LOG", "p1-invalid-parent-log");
        for name in HISTORICAL {
            child.env_remove(name);
        }
        if pollution != "clean" {
            for name in &HISTORICAL[..5] {
                child.env(name, "p1-invalid-campaign");
            }
            if pollution == "l3c" || pollution == "both" {
                child.env(HISTORICAL[5], "p1-invalid-l3c");
            }
            if pollution == "l3d" || pollution == "both" {
                child.env(HISTORICAL[6], "p1-invalid-l3d");
            }
        }
        let output = child.output().expect("start isolated test process");
        assert!(
            output.status.success(),
            "{backend}/{pollution}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{backend}/{pollution}: child did not execute its scenario"
        );
        assert!(
            root.join("observation.txt").is_file(),
            "{backend}/{pollution}: worker did not produce an observation"
        );
    }
}
