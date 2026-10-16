use std::path::PathBuf;

use std::sync::OnceLock;
use whisper_adapters::{
    model_catalog::LocalModelCatalog,
    queue_store::{QueueStatus, QueueStore},
};
use whisper_core::{
    domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice},
    model_catalog::ModelId,
    ports::ImportRequest,
};

const KNOWN_TURBO: &str =
    r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";

fn root() -> PathBuf {
    PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime root"))
        .join(format!("legacy-{}", std::process::id()))
}

fn request(model_path: String, generation: Generation) -> ImportRequest {
    let sha = whisper_core::model_registry::model(ModelId::WhisperLargeV3Turbo)
        .artifact
        .sha256;
    let mut model_sha256 = [0; 32];
    for (index, byte) in model_sha256.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&sha[index * 2..index * 2 + 2], 16).unwrap();
    }
    ImportRequest {
        job_id: JobId(77),
        generation,
        source_path: "source.wav".into(),
        source_sha256: Some([3; 32]),
        source_samples: Some(32_000),
        model_path,
        model_sha256,
        destination: root().to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    }
}

fn verified_catalog() -> &'static whisper_adapters::model_catalog::CatalogSnapshot {
    static SNAPSHOT: OnceLock<whisper_adapters::model_catalog::CatalogSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../models");
        LocalModelCatalog::new(path).inspect(3)
    })
}

#[test]
fn queued_legacy_turbo_path_is_resolved_in_memory_and_durable_record_is_unchanged() {
    let destination = root().join("queue-process");
    let mut queue = QueueStore::open(&destination).unwrap();
    let original = request(KNOWN_TURBO.into(), Generation::first());
    queue.enqueue(original.clone()).unwrap();

    let executable = queue
        .resolve_queued_with_catalog(
            original.job_id,
            original.generation,
            verified_catalog(),
            KNOWN_TURBO,
        )
        .unwrap();
    assert!(whisper_core::model_selection_app::is_known_historical_path(
        &executable.model_path,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../models/ggml-large-v3-turbo.bin")
            .to_string_lossy()
    ));
    assert_eq!(queue.entries()[0].request, original);
}

#[test]
fn unknown_queued_path_is_refused_with_an_explicit_diagnostic() {
    let destination = root().join("queue-unknown");
    let mut queue = QueueStore::open(&destination).unwrap();
    let unknown = request(r"C:\old\unrecognized.bin".into(), Generation::first());
    queue.enqueue(unknown.clone()).unwrap();

    let error = queue
        .resolve_queued_with_catalog(
            unknown.job_id,
            unknown.generation,
            verified_catalog(),
            KNOWN_TURBO,
        )
        .unwrap_err();
    assert!(error.contains("chemin de modèle inconnu"));
    assert_eq!(queue.entries()[0].request, unknown);
}

#[test]
fn explicit_resume_accepts_a_catalog_path_for_known_turbo_and_keeps_durable_path() {
    let destination = root().join("queue-resume");
    let mut queue = QueueStore::open(&destination).unwrap();
    let previous = request(KNOWN_TURBO.into(), Generation::first());
    queue.enqueue(previous.clone()).unwrap();
    queue
        .set_status(
            previous.job_id,
            previous.generation,
            QueueStatus::Interrupted,
        )
        .unwrap();
    let catalog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../models/ggml-large-v3-turbo.bin")
        .to_string_lossy()
        .into_owned();
    let next_generation = previous.generation.next().unwrap();
    let resumed = request(catalog_path, next_generation);

    queue
        .validate_resume_with_catalog(
            previous.job_id,
            previous.generation,
            &resumed,
            verified_catalog(),
            KNOWN_TURBO,
        )
        .unwrap();
    let durable = queue
        .resume_interrupted_with_catalog(
            previous.job_id,
            previous.generation,
            resumed,
            verified_catalog(),
            KNOWN_TURBO,
        )
        .unwrap();
    assert_eq!(durable.request.model_path, KNOWN_TURBO);
    assert_eq!(durable.status, QueueStatus::Running);
}
