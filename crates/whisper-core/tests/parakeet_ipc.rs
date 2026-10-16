use whisper_core::parakeet_ipc::{
    Command, Envelope, Event, PARAKEET_IPC_VERSION, PARAKEET_MAX_AUDIO_SAMPLES,
    PARAKEET_MAX_FRAME_BYTES, PARAKEET_PROFILE_IMPORT_SINGLE_PASS, Word, WorkerIdentity,
};

#[test]
fn parakeet_protocol_is_versioned_and_caps_are_explicit() {
    let command = Envelope::new(Command::Shutdown);
    assert_eq!(command.version, PARAKEET_IPC_VERSION);
    assert_eq!(PARAKEET_MAX_FRAME_BYTES, 1_048_576);
    assert_eq!(PARAKEET_MAX_AUDIO_SAMPLES, 28_800_000);
}

#[test]
fn transcribe_command_carries_complete_build_runtime_profile_and_attempt_identity() {
    let identity = test_identity();
    let command = Envelope::new(Command::Transcribe {
        request_id: 11,
        job_id: "job-a".into(),
        generation: 4,
        attempt: 2,
        identity: identity.clone(),
        source_path: "audio.wav".into(),
        source_sha256: "ef".repeat(32),
    });
    assert_eq!(command.version, PARAKEET_IPC_VERSION);
    let Command::Transcribe {
        request_id,
        generation,
        attempt,
        identity: decoded_identity,
        ..
    } = command.message
    else {
        unreachable!()
    };
    assert_eq!((request_id, generation, attempt), (11, 4, 2));
    assert_eq!(decoded_identity, identity);
}

#[test]
fn result_contract_keeps_request_identity_and_owned_word_times() {
    let event = Event::Result {
        request_id: 7,
        job_id: "job-a".into(),
        generation: 3,
        attempt: 2,
        identity: test_identity(),
        source_sha256: "ab".repeat(32),
        duration_samples: 16_000,
        text: "bonjour".into(),
        words: vec![Word {
            text: "bonjour".into(),
            start_ms: 0,
            end_ms: 500,
        }],
    };
    let Event::Result {
        request_id,
        job_id,
        generation,
        attempt,
        identity,
        words,
        ..
    } = event
    else {
        unreachable!()
    };
    assert_eq!(
        (request_id, job_id.as_str(), generation, attempt),
        (7, "job-a", 3, 2)
    );
    assert_eq!(identity.profile, PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    assert_eq!(identity.build_sha256, "cd".repeat(32));
    assert_eq!(
        words[0],
        Word {
            text: "bonjour".into(),
            start_ms: 0,
            end_ms: 500
        }
    );
}

fn test_identity() -> WorkerIdentity {
    WorkerIdentity {
        model_sha256: "ab".repeat(32),
        runtime_sha256: "bc".repeat(32),
        build_sha256: "cd".repeat(32),
        profile: PARAKEET_PROFILE_IMPORT_SINGLE_PASS.into(),
    }
}
