use super::*;
use std::io::Cursor;

#[test]
fn oversized_and_truncated_frames_are_rejected_before_json_handling() {
    let mut oversized = Cursor::new(vec![b'x'; PARAKEET_MAX_FRAME_BYTES + 1]);
    assert!(
        read_frame(&mut oversized)
            .unwrap_err()
            .contains("IpcFrameTooLarge")
    );
    let mut truncated = Cursor::new(b"{\"version\":1".to_vec());
    assert!(
        read_frame(&mut truncated)
            .unwrap_err()
            .contains("IpcTruncated")
    );
}
