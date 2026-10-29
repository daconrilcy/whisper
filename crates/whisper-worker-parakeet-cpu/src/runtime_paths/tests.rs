use super::*;
#[cfg(feature = "parakeet-qualification-reference")]
use whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE;

#[cfg(not(feature = "parakeet-cuda"))]
#[test]
fn accepts_only_the_two_canonical_l2_test_roots() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let l2a_roots = allowed_runtime_roots_for(&repository, &["l2a-1", "l2a-2"]);
    let l2a1 = repository
        .join("target/parakeet-tests/l2/l2a-1")
        .canonicalize()
        .unwrap();
    let l2a2 = repository
        .join("target/parakeet-tests/l2/l2a-2")
        .canonicalize()
        .unwrap();
    let l2b = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .unwrap();
    let l3a = repository
        .join("target/parakeet-tests/l3/l3a")
        .canonicalize()
        .unwrap();
    assert_eq!(l2a_roots, vec![l2a1.clone(), l2a2.clone()]);
    assert_eq!(
        allowed_runtime_roots(&repository).contains(&l2a1),
        !cfg!(feature = "parakeet-personal")
    );
    assert_eq!(
        allowed_runtime_roots(&repository).contains(&l2a2),
        !cfg!(feature = "parakeet-personal")
    );
    assert_eq!(
        allowed_runtime_roots(&repository).contains(&l2b),
        cfg!(feature = "parakeet-qualification-reference") && !cfg!(feature = "parakeet-personal")
    );
    assert!(!allowed_runtime_roots(&repository).contains(&l3a));
    assert!(!allowed_runtime_roots(&repository).contains(&repository));
}

#[cfg(all(feature = "parakeet-cuda", not(feature = "parakeet-personal")))]
#[test]
fn gpu_worker_allows_only_the_canonical_l3_runtime_roots() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let l3a = repository
        .join("target/parakeet-tests/l3/l3a")
        .canonicalize()
        .unwrap();
    let l3a_expected = repository.join("target/parakeet-tests/l3/l3a");
    let l3b_expected = repository.join("target/parakeet-tests/l3/l3b");
    let l2a1 = repository
        .join("target/parakeet-tests/l2/l2a-1")
        .canonicalize()
        .unwrap();
    let l2b = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .unwrap();
    assert_eq!(
        allowed_runtime_root_candidates(&repository),
        vec![l3a_expected, l3b_expected]
    );
    assert!(allowed_runtime_roots(&repository).contains(&l3a));
    assert!(!allowed_runtime_roots(&repository).contains(&l2a1));
    assert!(!allowed_runtime_roots(&repository).contains(&l2b));
}

#[cfg(feature = "parakeet-qualification-reference")]
#[test]
fn qualification_profiles_are_confined_to_l2b() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let l2a2 = repository
        .join("target/parakeet-tests/l2/l2a-2")
        .canonicalize()
        .unwrap();
    let l2b = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .unwrap();
    assert_ne!(l2a2, l2b);
    assert!(qualification_root_is_valid(
        PROFILE_QUALIFICATION_REFERENCE,
        &l2b,
        &repository
    ));
    assert!(!qualification_root_is_valid(
        PROFILE_QUALIFICATION_REFERENCE,
        &l2a2,
        &repository
    ));
    assert_eq!(
        qualification_root_is_valid(PARAKEET_PROFILE_IMPORT_SINGLE_PASS, &l2a2, &repository),
        !cfg!(feature = "parakeet-cuda") && !cfg!(feature = "parakeet-personal")
    );
}
