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
        source_sha256: Some([0; 32]),
        source_samples: Some(1),
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
    let source_path = root.join("source.mp3");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&source_path, b"source stays intact").unwrap();
    let source_before = std::fs::read(&source_path).unwrap();
    let mut first = request(1, &source_path.to_string_lossy());
    first.destination = root.to_string_lossy().into_owned();
    let mut second = request(2, "two.wav");
    second.destination = root.to_string_lossy().into_owned();
    let mut queue = QueueStore::open(&root).unwrap();
    let mut unidentified = request(9, "unidentified.mp3");
    unidentified.source_sha256 = None;
    assert!(queue.enqueue(unidentified).is_err());
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

    use std::io::Write;
    let queue_path = root.join("transcriptions").join("queue.jsonl");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&queue_path)
        .unwrap()
        .write_all(b"{\"op\":\"upsert\",\"version\":")
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
    assert!(
        std::fs::read(&queue_path).unwrap().ends_with(b"\n"),
        "an unacknowledged partial tail is removed before the next durable append"
    );
    let mut queue = QueueStore::open(&root).unwrap();
    let mut resumed_request = first_entry.request.clone();
    resumed_request.generation = resumed_request.generation.next().unwrap();
    let resumed = queue
        .resume_interrupted(
            first_entry.request.job_id,
            first_entry.request.generation,
            resumed_request.clone(),
        )
        .unwrap();
    assert_eq!(
        resumed.sequence, first_entry.sequence,
        "resume preserves FIFO order"
    );
    assert_eq!(resumed.request.generation, resumed_request.generation);
    assert_eq!(resumed.status, QueueStatus::Running);
    assert!(
        queue
            .remove(resumed.request.job_id, resumed.request.generation)
            .is_err()
    );
    queue
        .set_status(
            resumed.request.job_id,
            resumed.request.generation,
            QueueStatus::Interrupted,
        )
        .unwrap();
    queue
        .remove(resumed.request.job_id, resumed.request.generation)
        .unwrap();
    assert_eq!(std::fs::read(&source_path).unwrap(), source_before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resume_validation_rejects_later_fifo_row_without_advancing_any_generation() {
    let root = std::env::temp_dir().join(format!(
        "whisper-l02-resume-fifo-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut first = request(31, "first.wav");
    first.destination = root.to_string_lossy().into_owned();
    let mut second = request(32, "second.wav");
    second.destination = root.to_string_lossy().into_owned();
    let mut queue = QueueStore::open(&root).unwrap();
    let first_entry = queue.enqueue(first.clone()).unwrap();
    let second_entry = queue.enqueue(second.clone()).unwrap();
    queue
        .set_status(first.job_id, first.generation, QueueStatus::Interrupted)
        .unwrap();
    queue
        .set_status(second.job_id, second.generation, QueueStatus::Interrupted)
        .unwrap();
    let mut later_generation = second.clone();
    later_generation.generation = second.generation.next().unwrap();
    let before_refusal = queue.entries();
    assert!(
        queue
            .validate_resume(second.job_id, second.generation, &later_generation)
            .unwrap_err()
            .contains("FIFO")
    );
    assert_eq!(QueueStore::open(&root).unwrap().entries(), before_refusal);
    let next_generation_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", second.job_id.0))
        .join(later_generation.generation.get().to_string());
    assert!(
        !next_generation_dir.exists(),
        "FIFO refusal creates no pending generation"
    );

    let mut first_next = first.clone();
    first_next.generation = first.generation.next().unwrap();
    queue
        .validate_resume(first.job_id, first.generation, &first_next)
        .unwrap();
    let resumed = queue
        .resume_interrupted(first.job_id, first.generation, first_next.clone())
        .unwrap();
    assert_eq!(resumed.sequence, first_entry.sequence);
    assert_eq!(resumed.request.generation, first_next.generation);
    assert_eq!(QueueStore::open(&root).unwrap().entries()[0], resumed);
    assert_eq!(second_entry.sequence, 2);
    std::fs::remove_dir_all(root).unwrap();
}
