use super::*;

#[test]
fn frames_and_limits_are_preserved() {
    let mut input = b"1\n2\n".as_slice();
    assert_eq!(read_frame::<u32>(&mut input, None).unwrap(), Some(1));
    assert_eq!(read_frame::<u32>(&mut input, None).unwrap(), Some(2));
    assert_eq!(read_frame::<u32>(&mut input, None).unwrap(), None);
    assert_eq!(
        read_frame::<u32>(&mut b"1".as_slice(), None)
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert_eq!(
        read_frame::<u32>(&mut b"x\n".as_slice(), None)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let mut out = Vec::new();
    write_frame(&mut out, &"x".repeat(MAX_FRAME_BYTES - 3), None).unwrap();
    assert_eq!(out.len(), MAX_FRAME_BYTES);
    assert_eq!(
        read_frame::<String>(&mut out.as_slice(), None)
            .unwrap()
            .unwrap()
            .len(),
        MAX_FRAME_BYTES - 3
    );
    let mut out = Vec::new();
    assert_eq!(
        write_frame(&mut out, &"x".repeat(MAX_FRAME_BYTES - 2), None)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert!(out.is_empty());
    let mut too_large = vec![b'x'; MAX_FRAME_BYTES];
    too_large.push(b'\n');
    assert_eq!(
        read_frame::<serde_json::Value>(&mut too_large.as_slice(), None)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn serialization_and_io_errors_are_propagated() {
    struct Refused;
    impl serde::Serialize for Refused {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("refused"))
        }
    }
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Vec::new();
    assert!(write_frame(&mut out, &Refused, None).is_err());
    assert!(out.is_empty());
    assert_eq!(
        write_frame(&mut Broken, &"ok", None).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}
