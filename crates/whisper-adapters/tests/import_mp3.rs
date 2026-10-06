use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use whisper_adapters::{archive::sha256_file, worker_ipc::AsyncImportIo};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, JobState, LanguageChoice,
};

#[test]
fn real_mp3_runs_through_durable_choice_cpu_worker_and_txt_srt_archive() {
    let Some(source) = std::env::var_os("WHISPER_L02_MP3_FIXTURE").map(PathBuf::from) else {
        eprintln!("V-IMPORT-MP3 NOT RUN: set WHISPER_L02_MP3_FIXTURE to a Q-07 profile fixture");
        return;
    };
    let Some(model) = std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from) else {
        eprintln!("V-IMPORT-MP3 NOT RUN: set WHISPER_L01_MODEL to the approved D19 model");
        return;
    };
    assert!(source.is_file());
    assert!(model.is_file());
    let source_hash = sha256_file(&source).unwrap();
    let destination = std::env::temp_dir().join(format!(
        "whisper-l02-mp3-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(
        worker.is_file(),
        "CPU worker binary must exist at {}",
        worker.display()
    );
    let mut app = ImportApplication::new(AsyncImportIo::start(worker));
    let job_id = JobId(((std::process::id() as u128) << 64) | 2);
    let request = ImportRequest {
        job_id,
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: [
            0x1f, 0xc7, 0x0f, 0x77, 0x4d, 0x38, 0xeb, 0x16, 0x99, 0x93, 0xac, 0x39, 0x1e, 0xea,
            0x35, 0x7e, 0xf4, 0x7c, 0x88, 0x75, 0x7e, 0xf7, 0x2e, 0xe5, 0x94, 0x38, 0x79, 0xb7,
            0xe8, 0xe2, 0xbc, 0x69,
        ],
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    };
    app.dispatch(AppCommand::EnqueueImport { request }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let queued = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some(entry) = view.queue.first() {
            break entry.request.clone();
        }
        assert!(
            Instant::now() < deadline,
            "durable queue acknowledgment timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    app.dispatch(AppCommand::ProcessQueued { request: queued })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15 * 60);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Complete)
        {
            break;
        }
        if view
            .active_job
            .is_some_and(|(_, state)| matches!(state, JobState::Recoverable | JobState::Cancelled))
        {
            panic!("MP3 import did not complete: {:?}", view.message);
        }
        assert!(
            Instant::now() < deadline,
            "MP3 import timed out: {:?}",
            view.message
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        sha256_file(&source).unwrap(),
        source_hash,
        "source remains unchanged"
    );
    let history = whisper_adapters::archive::ArchiveStore::scan(&destination).unwrap();
    assert!(
        history
            .iter()
            .any(|item| item.job_id == job_id && item.complete),
        "verified TXT/SRT archive is published"
    );
    std::fs::remove_dir_all(destination).unwrap();
}
