use super::*;

#[test]
fn personal_worker_accepts_only_published_root_or_a_direct_staging_child() {
    let personal = Path::new(r"C:\Users\test\AppData\Local\Whisper");
    assert!(personal_runtime_path_is_allowed(personal, personal));
    assert!(personal_runtime_path_is_allowed(
        Path::new(r"C:\Users\test\AppData\Local\Whisper\.staging\candidate-1"),
        personal
    ));
    assert!(!personal_runtime_path_is_allowed(
        Path::new(r"C:\dev\whisper\target\parakeet-tests\l6-perso"),
        personal
    ));
    assert!(!personal_runtime_path_is_allowed(
        Path::new(r"C:\Users\test\AppData\Local\Whisper\.staging\candidate-1\nested"),
        personal
    ));
}
