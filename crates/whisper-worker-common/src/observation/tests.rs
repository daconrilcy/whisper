use std::sync::Mutex;

static EVENTS: Mutex<Vec<(String, serde_json::Value)>> = Mutex::new(Vec::new());
fn observe(stage: &str, fields: serde_json::Value) {
    EVENTS.lock().unwrap().push((stage.to_owned(), fields));
}

#[test]
fn observation_is_optional_and_feature_gated() {
    EVENTS.lock().unwrap().clear();
    let mut output = Vec::new();
    crate::ipc::write_frame(&mut output, &42, None).unwrap();
    assert!(EVENTS.lock().unwrap().is_empty());
    crate::ipc::write_frame(&mut output, &42, Some(observe)).unwrap();
    crate::ipc::read_frame::<u32>(&mut b"42\n".as_slice(), Some(observe)).unwrap();
    let events = EVENTS.lock().unwrap();
    #[cfg(not(feature = "l01-memory-qualification"))]
    assert!(events.is_empty());
    #[cfg(feature = "l01-memory-qualification")]
    {
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "ipc.serialize.buffer");
        assert_eq!(events[0].1["serialized_payload_len_bytes"], 2);
        assert_eq!(events[1].0, "ipc.read_frame.buffer");
        assert_eq!(events[1].1["frame_payload_len_bytes"], 2);
    }
}

#[test]
fn decoder_observer_preserves_stages() {
    let path =
        std::env::temp_dir().join(format!("worker-common-observer-{}.wav", std::process::id()));
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&68_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&16_000_u32.to_le_bytes());
    bytes.extend_from_slice(&32_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&32_u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 32]);
    std::fs::write(&path, bytes).unwrap();
    // The observer is a function pointer; collect this test in its own static store.
    DECODER_EVENTS.lock().unwrap().clear();
    crate::decoder::decode_wav_windows(&path, |_, _| Ok(()), None).unwrap();
    assert!(DECODER_EVENTS.lock().unwrap().is_empty());
    crate::decoder::decode_wav_windows(&path, |_, _| Ok(()), Some(observe_decoder)).unwrap();
    let observed = DECODER_EVENTS.lock().unwrap();
    #[cfg(not(feature = "l01-memory-qualification"))]
    assert!(observed.is_empty());
    #[cfg(feature = "l01-memory-qualification")]
    for required in [
        "codec.ready",
        "codec.packet.read",
        "codec.sample_buffer.live",
        "pcm.window.callback.begin",
        "pcm.window.callback.complete",
        "codec.complete",
    ] {
        assert!(
            observed.iter().any(|(stage, _)| stage == required),
            "missing stage {required}"
        );
    }
}
static DECODER_EVENTS: Mutex<Vec<(String, serde_json::Value)>> = Mutex::new(Vec::new());
fn observe_decoder(stage: &str, fields: serde_json::Value) {
    DECODER_EVENTS
        .lock()
        .unwrap()
        .push((stage.to_owned(), fields));
}
