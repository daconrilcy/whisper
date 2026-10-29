#[cfg(test)]
mod parakeet_stopped_ack_tests {
    use super::{ParakeetStoppedExit, parakeet_stopped_exit};

    #[test]
    fn stopped_ack_waits_for_exit_then_allows_completion() {
        assert_eq!(parakeet_stopped_exit(None), ParakeetStoppedExit::Wait);
        assert_eq!(
            parakeet_stopped_exit(Some(true)),
            ParakeetStoppedExit::Success
        );
        assert_eq!(
            parakeet_stopped_exit(Some(false)),
            ParakeetStoppedExit::Failure
        );
    }
}
