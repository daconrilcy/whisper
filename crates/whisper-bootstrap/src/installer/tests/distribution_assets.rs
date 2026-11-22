use super::super::assets::DUAL_FILES;

#[test]
fn dual_cpu_asset_table_is_complete_and_excludes_gpu_resources() {
    assert_eq!(DUAL_FILES.len(), 17);
    let mut destinations = Vec::new();
    let mut dynamic_identities = Vec::new();
    for record in DUAL_FILES {
        let fields = record.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 3);
        assert!(!fields[0].contains("gpu") && !fields[1].contains("gpu"));
        assert!(!fields[0].contains("cuda") && !fields[1].contains("cuda"));
        destinations.push(fields[1]);
        if fields[2].starts_with("WHISPER_DUAL_") {
            dynamic_identities.push(fields[2]);
        } else {
            assert_eq!(fields[2].len(), 64);
            assert!(fields[2].bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }
    destinations.sort_unstable();
    destinations.dedup();
    assert_eq!(destinations.len(), DUAL_FILES.len());
    dynamic_identities.sort_unstable();
    assert_eq!(
        dynamic_identities,
        [
            "WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256",
            "WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256",
            "WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256",
        ]
    );
}
