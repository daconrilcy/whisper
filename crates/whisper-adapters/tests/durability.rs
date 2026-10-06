use std::time::{SystemTime, UNIX_EPOCH};
use whisper_adapters::{
    queue_store::{QueueStatus, QueueStore},
    recovery,
};
use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice,
    ports::{ImportQueueStatus, ImportRequest},
};

fn request(job: u128, source: &str) -> ImportRequest {
    ImportRequest {
        job_id: JobId(job),
        generation: Generation::first(),
        source_path: source.into(),
        source_sha256: None,
        source_samples: None,
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: String::new(),
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    }
}

#[test]
fn queue_ack_replays_fifo_and_running_work_recovers_without_launch() {
    let root = std::env::temp_dir().join(format!(
        "whisper-l02-queue-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut first = request(1, "one.mp3");
    first.destination = root.to_string_lossy().into_owned();
    let mut second = request(2, "two.wav");
    second.destination = root.to_string_lossy().into_owned();
    let mut queue = QueueStore::open(&root).unwrap();
    let first_entry = queue.enqueue(first).unwrap();
    queue.enqueue(second).unwrap();
    assert_eq!(
        queue
            .entries()
            .iter()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    queue
        .set_status(
            first_entry.request.job_id,
            first_entry.request.generation,
            QueueStatus::Running,
        )
        .unwrap();

    let restored = recovery::scan(&root).unwrap();
    assert_eq!(restored.queue[0].status, ImportQueueStatus::Interrupted);
    assert_eq!(restored.queue[1].status, ImportQueueStatus::AwaitingChoice);
    let reopened = QueueStore::open(&root).unwrap();
    assert_eq!(reopened.entries(), restored.queue);
    assert_eq!(
        recovery::scan(&root).unwrap(),
        restored,
        "recovery scan is idempotent"
    );
    std::fs::remove_dir_all(root).unwrap();
}
