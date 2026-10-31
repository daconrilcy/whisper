use serde_json::{Value, json};
use whisper_core::parakeet_ipc::{Command, Envelope, ExpectedGpu, WorkerIdentity};

const LEGACY_WITHOUT_GPU: &str = r#"{
    "version": 1,
    "message": {
        "type": "transcribe",
        "request_id": 11,
        "job_id": "job-a",
        "generation": 4,
        "attempt": 2,
        "identity": {
            "model_sha256": "model-hash",
            "runtime_sha256": "runtime-hash",
            "build_sha256": "build-hash",
            "profile": "import-single-pass"
        },
        "source_path": "audio.wav",
        "source_sha256": "source-hash"
    }
}"#;

fn expected_identity() -> WorkerIdentity {
    WorkerIdentity {
        model_sha256: "model-hash".into(),
        runtime_sha256: "runtime-hash".into(),
        build_sha256: "build-hash".into(),
        profile: "import-single-pass".into(),
    }
}

fn expected_transcribe(expected_gpu: Option<ExpectedGpu>) -> Envelope<Command> {
    Envelope::new(Command::Transcribe {
        request_id: 11,
        job_id: "job-a".into(),
        generation: 4,
        attempt: 2,
        identity: Box::new(expected_identity()),
        source_path: "audio.wav".into(),
        source_sha256: "source-hash".into(),
        expected_gpu,
    })
}

#[test]
fn transcribe_v1_accepts_legacy_and_explicit_gpu_fields() {
    let absent: Value = serde_json::from_str(LEGACY_WITHOUT_GPU).unwrap();
    let mut null = absent.clone();
    null["message"]["expected_gpu"] = Value::Null;
    let mut present = absent.clone();
    present["message"]["expected_gpu"] = json!({
        "pci_bus_id": "00000000:01:00.0",
        "uuid": "GPU-test"
    });

    for (fixture, expected_gpu) in [
        (absent, None),
        (null.clone(), None),
        (
            present.clone(),
            Some(ExpectedGpu {
                pci_bus_id: "00000000:01:00.0".into(),
                uuid: "GPU-test".into(),
            }),
        ),
    ] {
        let parsed: Envelope<Command> = serde_json::from_value(fixture).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed, expected_transcribe(expected_gpu));
    }

    assert_eq!(
        serde_json::to_value(expected_transcribe(None)).unwrap(),
        null
    );
    assert_eq!(
        serde_json::to_value(expected_transcribe(Some(ExpectedGpu {
            pci_bus_id: "00000000:01:00.0".into(),
            uuid: "GPU-test".into(),
        })))
        .unwrap(),
        present
    );
}

#[test]
fn shutdown_v1_has_the_existing_wire_shape() {
    let fixture = json!({ "version": 1, "message": { "type": "shutdown" } });
    let parsed: Envelope<Command> = serde_json::from_value(fixture.clone()).unwrap();
    assert_eq!(parsed, Envelope::new(Command::Shutdown));
    assert_eq!(serde_json::to_value(parsed).unwrap(), fixture);
}
