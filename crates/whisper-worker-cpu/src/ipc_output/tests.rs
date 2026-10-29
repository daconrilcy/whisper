use super::*;

#[test]
fn writer_accepts_limit_and_rejects_limit_plus_one_without_growing_past_it() {
    let exact = "x".repeat(MAX_IPC_FRAME_BYTES - 3);
    let mut output = Vec::new();
    write_frame(&mut output, &exact).unwrap();
    assert_eq!(output.len(), MAX_IPC_FRAME_BYTES);

    let oversized = "x".repeat(MAX_IPC_FRAME_BYTES - 2);
    assert_eq!(
        write_frame(&mut Vec::new(), &oversized).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}
