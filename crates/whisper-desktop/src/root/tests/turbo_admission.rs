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
