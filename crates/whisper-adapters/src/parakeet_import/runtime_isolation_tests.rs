include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/support/parakeet_runtime_isolation.rs"
));

#[test]
fn personal_session_is_independent_of_historical_environment() {
    run_matrix(ParakeetProcess::spawn_live);
}
