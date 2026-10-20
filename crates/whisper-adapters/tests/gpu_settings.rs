use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use whisper_adapters::path_settings::WindowsPathPort;
use whisper_core::{
    ComputeChoice, MicrophoneChoice,
    gpu_inventory::{
        GpuAvailability, GpuIdentityMatch, GpuObservation, GpuPresence, GpuQualifiedObservation,
        GpuRankSource, GpuState, GpuSupport, GpuWindowsRank, qualify,
    },
    gpu_inventory_app::GpuQualifiedInventoryState,
    gpu_selection::{SelectionResolution, SessionCandidate, resolve},
    path_settings::{PathDefaults, PathField, PathSettingsApp, SaveStatus, StoredPaths},
};

fn isolated_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir()
        .join("gpu-selection-tests")
        .join(format!("{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn defaults(dir: &Path) -> PathDefaults {
    PathDefaults {
        audio_directory: dir.join("audio").to_string_lossy().into_owned(),
        model_path: dir.join("model.bin").to_string_lossy().into_owned(),
        destination: dir.join("output").to_string_lossy().into_owned(),
    }
}

fn wait_for_latest_save(
    app: &mut PathSettingsApp,
    revision: u64,
) -> (Result<(), String>, Option<String>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut notice = None;
    while std::time::Instant::now() < deadline {
        notice = app.poll_notice().or(notice);
        match app.save_status() {
            SaveStatus::Saved { revision: saved } if *saved == revision => {
                return (Ok(()), notice);
            }
            SaveStatus::Failed {
                revision: failed,
                error,
            } if *failed == revision => return (Err(error.clone()), notice),
            _ => thread::sleep(Duration::from_millis(10)),
        }
    }
    panic!("révision {revision} non acquittée");
}

#[test]
fn old_json_migrates_additively_and_reopens_with_revision_acknowledgement() {
    let dir = isolated_dir("migration");
    let file = dir.join("settings.json");
    let defaults_value = defaults(&dir);
    fs::create_dir_all(&defaults_value.audio_directory).unwrap();
    fs::write(&defaults_value.model_path, b"model marker").unwrap();
    fs::create_dir_all(&defaults_value.destination).unwrap();
    let model_path = defaults_value.model_path.clone();
    let destination = defaults_value.destination.clone();
    let old_json = serde_json::json!({
        "version": 1,
        "audio_directory": defaults_value.audio_directory,
        "model_path": model_path.clone(),
        "destination": destination.clone(),
        "microphone_id": "mic:one",
    });
    fs::write(&file, serde_json::to_vec(&old_json).unwrap()).unwrap();
    let mut app = PathSettingsApp::new(
        defaults(&dir),
        Box::new(WindowsPathPort::new_at(file.clone())),
    );
    assert_eq!(app.gpu_preference(), None);
    app.set_gpu_preference(Some("PCI\\VEN_10DE&DEV_A".into()));
    let revision = match app.save_status() {
        SaveStatus::Pending { revision } => *revision,
        status => panic!("état de sauvegarde inattendu : {status:?}"),
    };
    wait_for_latest_save(&mut app, revision).0.unwrap();

    let stored: StoredPaths = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(stored.microphone_id.as_deref(), Some("mic:one"));
    assert_eq!(stored.gpu_device_id.as_deref(), Some("PCI\\VEN_10DE&DEV_A"));
    assert_eq!(stored.model_path, model_path);
    assert_eq!(stored.destination, destination);

    let reopened = PathSettingsApp::new(defaults(&dir), Box::new(WindowsPathPort::new_at(file)));
    assert_eq!(reopened.gpu_preference(), Some("PCI\\VEN_10DE&DEV_A"));
    assert_eq!(app.save_status(), &SaveStatus::Saved { revision });
}

#[test]
fn temporary_substitution_uses_q_then_restores_preferred_p() {
    let row = |identity: &str, rank| GpuQualifiedObservation {
        observation: GpuObservation {
            identity: identity.into(),
            name: Some("Même nom".into()),
            state: Some(GpuState::Ready),
            presence: GpuPresence::Current,
        },
        windows_rank: Some(GpuWindowsRank {
            ordinal: rank,
            source: GpuRankSource::DxgiEnumeration,
        }),
        identity_match: GpuIdentityMatch::Unique,
        support: GpuSupport::ConfirmedSupported,
        availability: GpuAvailability::Available,
        causes: vec![],
    };
    let with_p =
        GpuQualifiedInventoryState::Complete(qualify(vec![row("gpu:P", 0), row("gpu:Q", 1)], None));
    assert!(matches!(
        resolve(ComputeChoice::Gpu, Some("gpu:P"), &with_p),
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate))
            if candidate.identity == "gpu:P" && !candidate.preference_substituted
    ));
    let without_p = GpuQualifiedInventoryState::Complete(qualify(vec![row("gpu:Q", 0)], None));
    assert!(matches!(
        resolve(ComputeChoice::Gpu, Some("gpu:P"), &without_p),
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate))
            if candidate.identity == "gpu:Q" && candidate.preference_substituted
    ));
    assert!(matches!(
        resolve(ComputeChoice::Gpu, Some("gpu:P"), &with_p),
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate))
            if candidate.identity == "gpu:P" && !candidate.preference_substituted
    ));
}

#[test]
fn interleaved_path_microphone_and_gpu_edits_preserve_one_latest_snapshot() {
    let dir = isolated_dir("interleaved");
    let model = dir.join("model-updated.bin");
    let audio = dir.join("voice.wav");
    let output = dir.join("output-updated");
    fs::write(&model, b"model marker").unwrap();
    fs::write(&audio, b"audio marker").unwrap();
    fs::create_dir_all(&output).unwrap();
    let file = dir.join("settings.json");
    let mut app = PathSettingsApp::new(
        defaults(&dir),
        Box::new(WindowsPathPort::new_at(file.clone())),
    );

    app.model = model.to_string_lossy().into_owned();
    app.commit_manual(PathField::Model);
    app.audio = audio.to_string_lossy().into_owned();
    app.commit_manual(PathField::Audio);
    app.set_microphone_preference(&MicrophoneChoice::Device("mic:new".into()));
    app.destination = output.to_string_lossy().into_owned();
    app.commit_manual(PathField::Destination);
    app.set_gpu_preference(Some("gpu:preferred".into()));
    let revision = match app.save_status() {
        SaveStatus::Pending { revision } => *revision,
        status => panic!("état de sauvegarde inattendu : {status:?}"),
    };
    wait_for_latest_save(&mut app, revision).0.unwrap();

    let stored: StoredPaths = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
    assert_eq!(stored.model_path, model.to_string_lossy());
    assert_eq!(stored.destination, output.to_string_lossy());
    assert_eq!(stored.microphone_id.as_deref(), Some("mic:new"));
    assert_eq!(stored.gpu_device_id.as_deref(), Some("gpu:preferred"));
    assert_eq!(
        stored.audio_directory.as_deref(),
        Some(dir.to_string_lossy().as_ref())
    );
}

#[test]
fn write_failure_is_attributed_to_revision_and_reported() {
    let dir = isolated_dir("write-failure");
    let blocker = dir.join("blocker");
    fs::write(&blocker, b"not a directory").unwrap();
    let settings_path = blocker.join("settings.json");
    let mut app = PathSettingsApp::new(
        defaults(&dir),
        Box::new(WindowsPathPort::new_at(settings_path)),
    );
    app.set_gpu_preference(Some("gpu:preferred".into()));
    let revision = match app.save_status() {
        SaveStatus::Pending { revision } => *revision,
        status => panic!("état de sauvegarde inattendu : {status:?}"),
    };
    let (failure, notice) = wait_for_latest_save(&mut app, revision);
    let error = failure.unwrap_err();
    assert!(!error.is_empty());
    assert!(matches!(
        app.save_status(),
        SaveStatus::Failed { revision: failed, .. } if *failed == revision
    ));
    assert!(notice.is_some_and(|message| message.contains(&format!("révision {revision}"))));
}
