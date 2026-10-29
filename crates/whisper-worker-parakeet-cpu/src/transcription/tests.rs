use super::classify_native_error;

#[test]
fn native_error_classes_preserve_the_original_message() {
    for (message, code) in [
        ("InvalidTimestamp: raw bounds", "InvalidTimestamp"),
        ("NativeCallStarted: write failed", "IpcWrite"),
        ("native inference failed", "InferenceFailed"),
    ] {
        assert_eq!(
            classify_native_error(message.to_owned()),
            (code, message.to_owned())
        );
    }
}
