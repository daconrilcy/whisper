use super::*;

#[test]
fn turbo_admission_rejects_injected_parakeet_catalog_and_command() {
    let inherited_parakeet_config =
        |_: &str| Some(std::ffi::OsString::from("C:/inherited/parakeet-runtime"));
    assert!(
        super::super::configured_parakeet_worker_for_edition(true, inherited_parakeet_config)
            .is_none()
    );
    assert!(
        super::super::configured_gpu_parakeet_worker_for_edition(true, inherited_parakeet_config)
            .is_none()
    );
    let model_root = fixture_root("turbo-parakeet-admission");
    std::fs::create_dir_all(&model_root).unwrap();
    let io = RecordingIo::default();
    let effects = io.effects.clone();
    let snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
    let historical = "C:/known/turbo.bin".to_owned();
    let parakeet_path = model_root
        .join(
            model_registry::model(ModelId::ParakeetTdt06BV3)
                .artifact
                .file_name,
        )
        .to_string_lossy()
        .into_owned();
    let mut app = super::super::edition_admission::guarded_application_for_edition(
        io,
        &model_root,
        historical,
        admission_receiver(snapshot),
        true,
    );
    app.poll_admission_catalog();
    let result = app.admission_result(&AppCommand::StartImport {
        request: request(
            parakeet_path,
            model_sha(ModelId::ParakeetTdt06BV3),
            "archive".into(),
        ),
    });
    assert!(result.is_some_and(|admission| admission.is_err()));
    assert!(effects.lock().unwrap().is_empty());
}

#[test]
fn dual_cpu_distribution_routes_parakeet_only_through_private_cpu_assets() {
    let root = std::path::Path::new(r"C:\Users\tester\AppData\Local\WhisperTurbo");
    let external_desktop = std::path::Path::new(r"C:\validation\outside\whisper-desktop.exe");
    let turbo_worker = super::super::distribution_edition::worker_path(
        Some(root),
        Some(external_desktop),
        "whisper-worker-cpu.exe",
        true,
        true,
    );
    assert_eq!(turbo_worker, root.join("app/whisper-worker-cpu.exe"));
    assert_eq!(
        super::super::distribution_edition::private_install_root(
            Some(std::ffi::OsString::from(r"C:\Users\tester\AppData\Local")),
            true,
        ),
        Some(root.to_path_buf())
    );
    let cpu = super::super::personal_worker_paths(
        root,
        whisper_core::model_catalog::Backend::Cpu,
    );
    assert_eq!(cpu.executable, root.join("worker/whisper-worker-parakeet-cpu.exe"));
    assert_eq!(cpu.native_dir, root.join("native/cpu"));
    assert_eq!(cpu.model, root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"));
    let inherited = |name: &str| Some(std::ffi::OsString::from(format!("C:/injected/{name}")));
    assert!(super::super::configured_gpu_parakeet_worker_for_distribution(
        true, false, inherited
    )
    .is_none());

    let legacy_desktop = std::path::Path::new(r"C:\personal\app\whisper-desktop.exe");
    assert_eq!(
        super::super::distribution_edition::private_install_root(
            Some(std::ffi::OsString::from(r"C:\Users\tester\AppData\Local")),
            false,
        ),
        Some(std::path::PathBuf::from(r"C:\Users\tester\AppData\Local\Whisper"))
    );
    assert_eq!(
        super::super::distribution_edition::worker_path(
            None,
            Some(legacy_desktop),
            "whisper-worker-cpu.exe",
            true,
            false,
        ),
        legacy_desktop.parent().unwrap().join("whisper-worker-cpu.exe")
    );
}
