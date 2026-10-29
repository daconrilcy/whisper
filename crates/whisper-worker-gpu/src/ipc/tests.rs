use super::*;

#[test]
fn rejects_oversized_and_incomplete_frames_before_deserialization() {
    let mut oversized = vec![b'a'; MAX_FRAME_BYTES];
    oversized.push(b'\n');
    assert_eq!(
        read_frame::<serde_json::Value>(&mut oversized.as_slice())
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let mut incomplete = br#"{"version":2}"#.as_slice();
    assert_eq!(
        read_frame::<serde_json::Value>(&mut incomplete)
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn writer_accepts_limit_and_rejects_limit_plus_one() {
    let exact = "x".repeat(MAX_FRAME_BYTES - 3);
    let mut output = Vec::new();
    write_frame(&mut output, &exact).unwrap();
    assert_eq!(output.len(), MAX_FRAME_BYTES);

    let oversized = "x".repeat(MAX_FRAME_BYTES - 2);
    assert_eq!(
        write_frame(&mut Vec::new(), &oversized).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}
