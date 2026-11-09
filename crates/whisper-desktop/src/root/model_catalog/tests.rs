use super::*;
use std::{
    collections::VecDeque,
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use whisper_adapters::model_catalog::{
    artifact_sha256_full_read_count, cached_artifact_sha256, model_sha256_full_read_count,
};
use whisper_core::ports::{ImportEvent, ImportQueueEntry, ImportQueueStatus, PortError};
use whisper_core::{
    Generation, ImportEffect, ImportRequest, JobConfig, JobId, LanguageChoice, LiveRequest,
};

const WAIT: Duration = Duration::from_secs(10);
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "catalog-service-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for record in model_registry::MODELS {
            fs::write(root.join(record.artifact.file_name), b"synthetic model").unwrap();
        }
        Self(root)
    }
    fn path(&self, id: ModelId) -> PathBuf {
        self.0.join(model_registry::model(id).artifact.file_name)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ready_snapshot(
    root: &std::path::Path,
    generation: u64,
    b: ModelAvailability,
) -> CatalogSnapshot {
    let mut snapshot = uninspected_snapshot(root, generation);
    for entry in &mut snapshot.entries {
        entry.availability = if entry.id == ModelId::WhisperLargeV3Turbo {
            ModelAvailability::Ready
        } else {
            b
        };
        if entry.availability == ModelAvailability::Ready {
            entry.observed_size_bytes = Some(entry.expected_size_bytes);
            entry.observed_sha256 = Some(entry.expected_sha256.clone());
            entry.diagnostic = None;
        } else if entry.availability != ModelAvailability::Uninspected {
            entry.diagnostic = Some(format!("synthetic {:?}", entry.availability));
        }
    }
    snapshot
}

#[test]
fn composed_service_starts_complete_uninspected_and_reads_only_effective_parakeet_including_qualification()
 {
    let root = Root::new();
    let qualification = root.0.join("qualification-worker.bin");
    fs::write(&qualification, b"qualification resource").unwrap();
    let resource = qualification.clone();
    let (requests, catalog, admission) =
        start_catalog_service_with(root.0.clone(), move |catalog, id, generation| {
            let snapshot = catalog.inspect_model(id, generation);
            if id == ModelId::ParakeetTdt06BV3 {
                cached_artifact_sha256(&resource).unwrap();
            }
            snapshot
        });
    let initial = catalog.recv_timeout(WAIT).unwrap();
    assert_eq!(initial.entries.len(), ModelId::ALL.len());
    assert!(
        initial
            .entries
            .iter()
            .all(|entry| entry.availability == ModelAvailability::Uninspected
                && entry.observed_sha256.is_none())
    );
    admission.recv_timeout(WAIT).unwrap();
    assert_eq!(
        model_sha256_full_read_count(&root.path(ModelId::WhisperLargeV3Turbo)).unwrap(),
        0
    );
    assert_eq!(
        model_sha256_full_read_count(&root.path(ModelId::ParakeetTdt06BV3)).unwrap(),
        0
    );
    assert_eq!(artifact_sha256_full_read_count(&qualification).unwrap(), 0);
    requests.select(ModelId::ParakeetTdt06BV3, 1).unwrap();
    let observed = catalog.recv_timeout(WAIT).unwrap();
    assert_eq!(observed.entries.len(), 2);
    assert_eq!(
        observed
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::WhisperLargeV3Turbo)
            .unwrap()
            .availability,
        ModelAvailability::Uninspected
    );
    assert_eq!(
        model_sha256_full_read_count(&root.path(ModelId::WhisperLargeV3Turbo)).unwrap(),
        0
    );
    assert_eq!(
        model_sha256_full_read_count(&root.path(ModelId::ParakeetTdt06BV3)).unwrap(),
        1
    );
    assert_eq!(artifact_sha256_full_read_count(&qualification).unwrap(), 1);
    requests.close();
}

#[test]
fn actual_service_preserves_missing_and_corrupt_states_and_never_hashes_turbo_for_parakeet() {
    for missing in [false, true] {
        let root = Root::new();
        if missing {
            fs::remove_file(root.path(ModelId::ParakeetTdt06BV3)).unwrap();
        }
        let (requests, catalog, _admission) = start_catalog_service(root.0.clone());
        catalog.recv_timeout(WAIT).unwrap();
        requests.select(ModelId::ParakeetTdt06BV3, 1).unwrap();
        let observed = catalog.recv_timeout(WAIT).unwrap();
        let entry = observed
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap();
        assert_eq!(
            entry.availability,
            if missing {
                ModelAvailability::Missing
            } else {
                ModelAvailability::Corrupt
            }
        );
        assert_eq!(
            model_sha256_full_read_count(&root.path(ModelId::WhisperLargeV3Turbo)).unwrap(),
            0
        );
        requests.close();
    }
}

#[test]
fn mailbox_coalesces_selection_and_deduplicates_commands_in_stable_priority_order() {
    let (requests, receiver) = catalog_inspection_channel();
    requests.select(ModelId::WhisperLargeV3Turbo, 1).unwrap();
    requests.command(ModelId::ParakeetTdt06BV3).unwrap();
    requests.command(ModelId::ParakeetTdt06BV3).unwrap();
    requests.select(ModelId::WhisperLargeV3Turbo, 2).unwrap();
    requests.select(ModelId::ParakeetTdt06BV3, 1).unwrap();
    assert_eq!(
        requests.selected_model(),
        Some(ModelId::WhisperLargeV3Turbo)
    );
    assert_eq!(
        receiver.next(Duration::ZERO).unwrap().unwrap(),
        CatalogInspectionNeed {
            model: ModelId::ParakeetTdt06BV3,
            version: 1
        }
    );
    assert_eq!(
        receiver.next(Duration::ZERO).unwrap().unwrap(),
        CatalogInspectionNeed {
            model: ModelId::WhisperLargeV3Turbo,
            version: 2
        }
    );
    assert!(receiver.next(Duration::ZERO).unwrap().is_none());
}

#[test]
fn newer_inspection_of_same_model_discards_old_result_without_reselecting() {
    let root = Root::new();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let mut calls = 0;
    let (requests, catalog, _admission) =
        start_catalog_service_with(root.0.clone(), move |catalog, id, generation| {
            calls += 1;
            if calls == 1 {
                entered_tx.send(()).unwrap();
                resume_rx.recv_timeout(WAIT).unwrap();
            }
            let mut snapshot = catalog.inspect_model(id, generation);
            snapshot.entries[0].diagnostic = Some(format!("observation {calls}"));
            snapshot
        });
    catalog.recv_timeout(WAIT).unwrap();
    requests.select(ModelId::ParakeetTdt06BV3, 1).unwrap();
    entered_rx.recv_timeout(WAIT).unwrap();
    requests.select(ModelId::ParakeetTdt06BV3, 2).unwrap();
    resume_tx.send(()).unwrap();
    let mut current = catalog.recv_timeout(WAIT).unwrap();
    assert_eq!(
        current
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap()
            .diagnostic
            .as_deref(),
        Some("observation 2")
    );
    assert!(requests.refresh_snapshot(&mut current));
    requests.close();
}

#[test]
fn identity_changed_during_observation_never_publishes_ready() {
    let root = Root::new();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let (requests, catalog, _admission) =
        start_catalog_service_with(root.0.clone(), move |catalog, id, generation| {
            let snapshot = ready_snapshot(catalog.root(), generation, ModelAvailability::Ready);
            entered_tx.send(()).unwrap();
            resume_rx.recv_timeout(WAIT).unwrap();
            CatalogSnapshot {
                generation,
                entries: snapshot
                    .entries
                    .into_iter()
                    .filter(|entry| entry.id == id)
                    .collect(),
            }
        });
    catalog.recv_timeout(WAIT).unwrap();
    requests.select(ModelId::ParakeetTdt06BV3, 1).unwrap();
    entered_rx.recv_timeout(WAIT).unwrap();
    fs::write(
        root.path(ModelId::ParakeetTdt06BV3),
        b"changed observable size",
    )
    .unwrap();
    resume_tx.send(()).unwrap();
    let snapshot = catalog.recv_timeout(WAIT).unwrap();
    let entry = snapshot
        .entries
        .iter()
        .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
        .unwrap();
    assert_eq!(entry.availability, ModelAvailability::Uninspected);
    assert!(entry.observed_sha256.is_none());
    assert!(entry.diagnostic.as_deref().unwrap().contains("modifié"));
    requests.close();
}

#[test]
fn closed_service_discards_inflight_observation_and_refuses_new_requests() {
    let root = Root::new();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::sync_channel(1);
    let (requests, catalog, _admission) =
        start_catalog_service_with(root.0.clone(), move |catalog, id, generation| {
            entered_tx.send(()).unwrap();
            resume_rx.recv_timeout(WAIT).unwrap();
            catalog.inspect_model(id, generation)
        });
    catalog.recv_timeout(WAIT).unwrap();
    requests.command(ModelId::ParakeetTdt06BV3).unwrap();
    entered_rx.recv_timeout(WAIT).unwrap();
    requests.close();
    resume_tx.send(()).unwrap();
    assert!(matches!(
        catalog.recv_timeout(WAIT),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
    assert!(requests.command(ModelId::ParakeetTdt06BV3).is_err());
}

#[test]
fn absent_identity_cannot_support_a_ready_publication() {
    let root = Root::new();
    fs::remove_file(root.path(ModelId::ParakeetTdt06BV3)).unwrap();
    let (requests, catalog, _admission) =
        start_catalog_service_with(root.0.clone(), |catalog, id, generation| {
            let mut snapshot = ready_snapshot(catalog.root(), generation, ModelAvailability::Ready);
            snapshot.entries.retain(|entry| entry.id == id);
            snapshot
        });
    catalog.recv_timeout(WAIT).unwrap();
    requests.command(ModelId::ParakeetTdt06BV3).unwrap();
    let snapshot = catalog.recv_timeout(WAIT).unwrap();
    let entry = snapshot
        .entries
        .iter()
        .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
        .unwrap();
    assert_eq!(entry.availability, ModelAvailability::Uninspected);
    assert!(
        entry
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("indisponible")
    );
    requests.close();
}

#[test]
fn parakeet_helpers_leave_uninspected_definitions_and_models_untouched() {
    let root = Root::new();
    let mut snapshot = uninspected_snapshot(&root.0, 1);
    qualify_parakeet_catalog_entry(&mut snapshot);
    apply_parakeet_key_availability(&mut snapshot, false);
    apply_parakeet_gpu_availability(&mut snapshot, false);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|entry| entry.availability == ModelAvailability::Uninspected)
    );
    for id in ModelId::ALL {
        assert_eq!(model_sha256_full_read_count(&root.path(id)).unwrap(), 0);
    }
}

#[test]
fn absent_catalog_observation_projects_as_uninspected_instead_of_missing_or_ready() {
    let snapshot = CatalogSnapshot {
        generation: 1,
        entries: vec![],
    };
    let engine = snapshot.engine_snapshot(vec![]);
    assert_eq!(engine.models.len(), ModelId::ALL.len());
    assert!(
        engine
            .models
            .iter()
            .all(|model| model.availability == ModelAvailability::Uninspected)
    );
    let projection = whisper_core::model_selection_app::ModelSelectionApp::new(engine)
        .projection(1)
        .unwrap();
    assert!(projection.models.iter().all(|model| !model.selectable
        && model.cause
            == Some(whisper_core::engine_capabilities::RefusalCause::ModelNotInspected)));
}

#[derive(Default)]
struct RecordingIo {
    effects: Arc<Mutex<Vec<ImportEffect>>>,
    events: Arc<Mutex<VecDeque<ImportEvent>>>,
}
impl ImportIoPort for RecordingIo {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
        self.effects.lock().unwrap().push(effect);
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
        Ok(self.events.lock().unwrap().pop_front())
    }
}

struct Harness {
    root: Root,
    app: DesktopAdmissionApplication<RecordingIo>,
    requester: CatalogInspectionSender,
    receiver: CatalogInspectionReceiver,
    catalog: Sender<CatalogSnapshot>,
    effects: Arc<Mutex<Vec<ImportEffect>>>,
    events: Arc<Mutex<VecDeque<ImportEvent>>>,
}
impl Harness {
    fn new() -> Self {
        let root = Root::new();
        let (requester, receiver) = catalog_inspection_channel();
        requester.select(ModelId::WhisperLargeV3Turbo, 1).unwrap();
        let selection = receiver.next(Duration::ZERO).unwrap().unwrap();
        assert!(receiver.publish(Some(selection), 1, [1, 0]).unwrap());
        let (catalog, rx) = mpsc::channel();
        catalog
            .send(ready_snapshot(&root.0, 1, ModelAvailability::Uninspected))
            .unwrap();
        let io = RecordingIo::default();
        let effects = Arc::clone(&io.effects);
        let events = Arc::clone(&io.events);
        let mut app = guarded_application(io, &root.0, "C:/legacy/turbo.bin".into(), rx)
            .with_inspection_requests(requester.clone());
        app.dispatch(AppCommand::Refresh).unwrap();
        Self {
            root,
            app,
            requester,
            receiver,
            catalog,
            effects,
            events,
        }
    }
    fn request(&self) -> ImportRequest {
        request_for(&self.root.0, 0xe601)
    }
    fn complete(&mut self, need: CatalogInspectionNeed, availability: ModelAvailability) {
        assert!(
            self.receiver
                .publish(Some(need), 2, [1, need.version])
                .unwrap()
        );
        self.catalog
            .send(ready_snapshot(&self.root.0, 2, availability))
            .unwrap();
        self.app.dispatch(AppCommand::Refresh).unwrap();
    }
}

fn registered_sha(value: &str) -> [u8; 32] {
    let mut sha = [0; 32];
    for (index, byte) in sha.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    sha
}

fn request_for(root: &std::path::Path, job: u128) -> ImportRequest {
    let record = model_registry::model(ModelId::ParakeetTdt06BV3);
    ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(job),
        generation: Generation::first(),
        source_path: "synthetic.wav".into(),
        source_sha256: None,
        source_samples: Some(16_000),
        model_path: root
            .join(record.artifact.file_name)
            .to_string_lossy()
            .into_owned(),
        model_sha256: registered_sha(record.artifact.sha256),
        destination: root.join("archive").to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
    }
}

fn queue_entry(request: &ImportRequest, resume: bool) -> ImportQueueEntry {
    ImportQueueEntry {
        sequence: request.job_id.0 as u64,
        request: request.clone(),
        status: if resume {
            ImportQueueStatus::Interrupted
        } else {
            ImportQueueStatus::Queued
        },
    }
}

fn explicit_retry_requires_new_click(resume: bool) {
    let mut harness = Harness::new();
    let request = harness.request();
    let status = if resume {
        ImportQueueStatus::Interrupted
    } else {
        ImportQueueStatus::Queued
    };
    harness
        .events
        .lock()
        .unwrap()
        .push_back(ImportEvent::Queue(vec![ImportQueueEntry {
            sequence: 1,
            request: request.clone(),
            status,
        }]));
    let before = harness.app.dispatch(AppCommand::Refresh).unwrap();
    let command = || {
        if resume {
            AppCommand::ResumeInterrupted {
                request: request.clone(),
            }
        } else {
            AppCommand::ProcessQueued {
                request: request.clone(),
            }
        }
    };
    harness
        .app
        .set_gpu_candidate(
            request.job_id,
            request.generation,
            Some(whisper_core::gpu_inventory::GpuRuntimeIdentity {
                generation: 1,
                pnp_instance_id: "old proof".into(),
                dxgi_rank: 0,
                luid: 1,
                pci_id: "test".into(),
                cuda_ordinal: 0,
                ggml_ordinal: 0,
            }),
        )
        .unwrap();
    let refused = harness.app.dispatch(command()).unwrap();
    assert!(refused.message.as_deref().unwrap().contains(if resume {
        "reprise"
    } else {
        "Traiter"
    }));
    assert_eq!(refused.queue, before.queue);
    assert_eq!(refused.active_generation, before.active_generation);
    assert!(harness.effects.lock().unwrap().is_empty());
    assert!(harness.app.pending_gpu_candidate.is_none());
    harness.app.dispatch(command()).unwrap();
    let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    assert_eq!(need.model, ModelId::ParakeetTdt06BV3);
    assert!(harness.receiver.next(Duration::ZERO).unwrap().is_none());
    harness.complete(need, ModelAvailability::Ready);
    assert_eq!(
        harness.requester.selected_model(),
        Some(ModelId::WhisperLargeV3Turbo)
    );
    assert!(harness.effects.lock().unwrap().is_empty());
    assert!(
        harness
            .app
            .dispatch(AppCommand::Refresh)
            .unwrap()
            .message
            .as_deref()
            .unwrap()
            .contains("Modèle vérifié")
    );
    harness
        .app
        .set_gpu_candidate(request.job_id, request.generation, None)
        .unwrap();
    harness.app.dispatch(command()).unwrap();
    let effects = harness.effects.lock().unwrap();
    assert!(matches!(
        effects.first(),
        Some(ImportEffect::SetGpuCandidate {
            candidate: None,
            ..
        })
    ));
    assert!(effects.iter().any(|effect| if resume {
        matches!(effect, ImportEffect::ResumeInterrupted { .. })
    } else {
        matches!(effect, ImportEffect::ProcessQueued(_))
    }));
}

#[test]
fn process_queued_b_with_a_selected_requests_b_then_requires_fresh_click_and_gpu_proof() {
    explicit_retry_requires_new_click(false);
}
#[test]
fn resume_interrupted_b_with_a_selected_requests_b_then_requires_fresh_click_and_gpu_proof() {
    explicit_retry_requires_new_click(true);
}

#[test]
fn missing_or_corrupt_command_model_reports_cause_without_restarting_or_repeating_inspection() {
    for availability in [ModelAvailability::Missing, ModelAvailability::Corrupt] {
        for resume in [false, true] {
            let mut harness = Harness::new();
            let request = harness.request();
            let command = || {
                if resume {
                    AppCommand::ResumeInterrupted {
                        request: request.clone(),
                    }
                } else {
                    AppCommand::ProcessQueued {
                        request: request.clone(),
                    }
                }
            };
            harness.app.dispatch(command()).unwrap();
            let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
            harness.complete(need, availability);
            assert!(
                harness
                    .app
                    .dispatch(AppCommand::Refresh)
                    .unwrap()
                    .message
                    .as_deref()
                    .unwrap()
                    .contains("synthetic")
            );
            harness.app.dispatch(command()).unwrap();
            assert!(harness.receiver.next(Duration::ZERO).unwrap().is_none());
            assert!(harness.effects.lock().unwrap().is_empty());
        }
    }
}

fn check_closing_before_observation(resume: bool) {
    let mut harness = Harness::new();
    let request = harness.request();
    harness
        .app
        .dispatch(queued_command(&request, resume))
        .unwrap();
    let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    harness.app.dispatch(AppCommand::RequestClose).unwrap();
    assert!(
        !harness
            .receiver
            .publish(Some(need), 2, [1, need.version])
            .unwrap()
    );
    harness
        .catalog
        .send(ready_snapshot(&harness.root.0, 2, ModelAvailability::Ready))
        .unwrap();
    harness.app.dispatch(AppCommand::Refresh).unwrap();
    assert!(
        harness
            .effects
            .lock()
            .unwrap()
            .iter()
            .all(|effect| !matches!(
                effect,
                ImportEffect::ProcessQueued(_)
                    | ImportEffect::ResumeInterrupted { .. }
                    | ImportEffect::StartWorker(_)
                    | ImportEffect::SetGpuCandidate { .. }
            ))
    );
}

fn live_request(request: &ImportRequest) -> LiveRequest {
    LiveRequest {
        job_id: request.job_id,
        generation: request.generation,
        model_path: request.model_path.clone(),
        model_sha256: request.model_sha256,
        destination: request.destination.clone(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: request.config.clone(),
    }
}

#[test]
fn every_explicit_import_and_configured_live_entry_requests_inspection_before_any_effect() {
    for route in 0..7 {
        let mut harness = Harness::new();
        let mut request = harness.request();
        request.config.compute = ComputeChoice::Cpu;
        let command = match route {
            0 => AppCommand::StartImport {
                request: request.clone(),
            },
            1 => AppCommand::EnqueueImport {
                request: request.clone(),
            },
            2 => AppCommand::ProcessQueued {
                request: request.clone(),
            },
            3 => AppCommand::ResumeInterrupted {
                request: request.clone(),
            },
            4 => AppCommand::StartLiveConfigured {
                request: live_request(&request),
            },
            5 => AppCommand::StartUnifiedLiveConfigured {
                request: live_request(&request),
            },
            _ => AppCommand::StartDirectLiveConfigured {
                request: live_request(&request),
            },
        };
        let view = harness.app.dispatch(command).unwrap();
        assert!(
            view.message
                .as_deref()
                .unwrap()
                .contains("Vérification du modèle")
        );
        assert!(view.active_job.is_none());
        assert!(harness.effects.lock().unwrap().is_empty());
        assert_eq!(
            harness
                .receiver
                .next(Duration::ZERO)
                .unwrap()
                .unwrap()
                .model,
            ModelId::ParakeetTdt06BV3
        );
    }
}

fn check_queue_removal_before_observation(resume: bool) {
    let mut harness = Harness::new();
    let request = harness.request();
    harness
        .events
        .lock()
        .unwrap()
        .push_back(ImportEvent::Queue(vec![ImportQueueEntry {
            sequence: 1,
            request: request.clone(),
            status: if resume {
                ImportQueueStatus::Interrupted
            } else {
                ImportQueueStatus::Queued
            },
        }]));
    harness.app.dispatch(AppCommand::Refresh).unwrap();
    harness
        .app
        .dispatch(queued_command(&request, resume))
        .unwrap();
    let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    harness
        .app
        .dispatch(AppCommand::RemoveQueued {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
    harness
        .events
        .lock()
        .unwrap()
        .push_back(ImportEvent::Queue(vec![]));
    harness.app.dispatch(AppCommand::Refresh).unwrap();
    let effects_before_publication = harness.effects.lock().unwrap().clone();
    harness.complete(need, ModelAvailability::Ready);
    assert!(harness.app.view().queue.is_empty());
    assert_eq!(*harness.effects.lock().unwrap(), effects_before_publication);
    assert_eq!(
        harness.requester.selected_model(),
        Some(ModelId::WhisperLargeV3Turbo)
    );
}

fn check_changed_selection_before_observation(resume: bool) {
    let mut harness = Harness::new();
    harness
        .app
        .dispatch(queued_command(&harness.request(), resume))
        .unwrap();
    let old_need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    harness
        .requester
        .select(ModelId::ParakeetTdt06BV3, 2)
        .unwrap();
    assert!(
        !harness
            .receiver
            .publish(Some(old_need), 2, [1, old_need.version])
            .unwrap()
    );
    let current = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    assert!(current.version > old_need.version);
    harness.complete(current, ModelAvailability::Ready);
    assert_eq!(
        harness.requester.selected_model(),
        Some(ModelId::ParakeetTdt06BV3)
    );
    assert!(harness.effects.lock().unwrap().is_empty());
}

fn check_generation_changed_during_inspection(resume: bool) {
    let mut harness = Harness::new();
    let request = harness.request();
    harness
        .app
        .dispatch(queued_command(&request, resume))
        .unwrap();
    let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    let mut newer = request.clone();
    newer.model_path = harness
        .root
        .path(ModelId::WhisperLargeV3Turbo)
        .to_string_lossy()
        .into_owned();
    newer.model_sha256 = registered_sha(
        model_registry::model(ModelId::WhisperLargeV3Turbo)
            .artifact
            .sha256,
    );
    newer.generation = request.generation.next().unwrap();
    newer.config.compute = ComputeChoice::Cpu;
    harness
        .app
        .inner
        .dispatch(AppCommand::StartImport { request: newer })
        .unwrap();
    harness.effects.lock().unwrap().clear();
    harness.complete(need, ModelAvailability::Ready);
    harness
        .app
        .set_gpu_candidate(request.job_id, request.generation, None)
        .unwrap();
    assert_eq!(
        harness.app.dispatch(queued_command(&request, resume)),
        Err(ApplicationError::InvalidCommand)
    );
    assert!(harness.effects.lock().unwrap().is_empty());
}

fn queued_command(request: &ImportRequest, resume: bool) -> AppCommand {
    if resume {
        AppCommand::ResumeInterrupted {
            request: request.clone(),
        }
    } else {
        AppCommand::ProcessQueued {
            request: request.clone(),
        }
    }
}

#[test]
fn closing_before_command_observation_cannot_start_it_when_a_response_arrives() {
    for resume in [false, true] {
        check_closing_before_observation(resume);
    }
}
#[test]
fn queue_removal_before_observation_does_not_replay_the_old_request() {
    for resume in [false, true] {
        check_queue_removal_before_observation(resume);
    }
}
#[test]
fn changed_selection_supersedes_same_model_observation_without_replaying_command() {
    for resume in [false, true] {
        check_changed_selection_before_observation(resume);
    }
}
#[test]
fn generation_changed_during_inspection_is_rechecked_before_any_new_gpu_effect() {
    for resume in [false, true] {
        check_generation_changed_during_inspection(resume);
    }
}

#[test]
fn disconnected_catalog_refuses_explicit_commands_instead_of_reusing_old_observations() {
    let mut harness = Harness::new();
    let request = harness.request();
    drop(harness.catalog);
    let view = harness
        .app
        .dispatch(queued_command(&request, false))
        .unwrap();
    assert!(view.message.as_deref().unwrap().contains("indisponible"));
    assert!(harness.effects.lock().unwrap().is_empty());
    assert!(harness.receiver.next(Duration::ZERO).unwrap().is_none());
}

#[test]
fn stop_during_observation_never_turns_its_result_into_a_start() {
    for resume in [false, true] {
        let mut harness = Harness::new();
        let request = harness.request();
        harness
            .app
            .dispatch(queued_command(&request, resume))
            .unwrap();
        let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
        let _ = harness.app.dispatch(AppCommand::Stop {
            job_id: request.job_id,
            generation: request.generation,
        });
        harness.complete(need, ModelAvailability::Ready);
        assert!(harness.effects.lock().unwrap().is_empty());
    }
}

#[test]
fn verified_command_b_is_available_while_newer_selection_a_is_still_pending() {
    let mut harness = Harness::new();
    harness
        .requester
        .select(ModelId::WhisperLargeV3Turbo, 2)
        .unwrap();
    let request = harness.request();
    harness
        .app
        .dispatch(queued_command(&request, false))
        .unwrap();
    let b = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    assert_eq!(b.model, ModelId::ParakeetTdt06BV3);
    harness.complete(b, ModelAvailability::Ready);
    assert!(
        harness
            .app
            .admission_result(&queued_command(&request, false))
            .unwrap()
            .is_ok()
    );
    assert!(harness.effects.lock().unwrap().is_empty());
    assert_eq!(
        harness
            .app
            .admission_snapshot
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::WhisperLargeV3Turbo)
            .unwrap()
            .availability,
        ModelAvailability::Uninspected
    );
    assert_eq!(
        harness
            .receiver
            .next(Duration::ZERO)
            .unwrap()
            .unwrap()
            .model,
        ModelId::WhisperLargeV3Turbo
    );
    assert_eq!(
        harness.requester.selected_model(),
        Some(ModelId::WhisperLargeV3Turbo)
    );
}

struct PausedObservations {
    entered_a: Receiver<()>,
    resume_a: mpsc::SyncSender<()>,
    entered_b: Receiver<()>,
    resume_b: mpsc::SyncSender<()>,
}

struct ObservationCleanup {
    requests: CatalogInspectionSender,
    resume_a: mpsc::SyncSender<()>,
    resume_b: mpsc::SyncSender<()>,
}

impl Drop for ObservationCleanup {
    fn drop(&mut self) {
        let _ = self.resume_a.try_send(());
        let _ = self.resume_b.try_send(());
        self.requests.close();
    }
}

fn paused_catalog(
    root: PathBuf,
    outcome: ModelAvailability,
) -> (
    CatalogInspectionSender,
    Receiver<CatalogSnapshot>,
    Receiver<CatalogSnapshot>,
    PausedObservations,
) {
    let (entered_a_tx, entered_a) = mpsc::sync_channel(1);
    let (resume_a, resume_a_rx) = mpsc::sync_channel(1);
    let (entered_b_tx, entered_b) = mpsc::sync_channel(1);
    let (resume_b, resume_b_rx) = mpsc::sync_channel(1);
    let mut first_observation = [true; 2];
    let (requests, catalog, admission) =
        start_catalog_service_with(root, move |catalog, id, generation| {
            let (index, entered, resume) = if id == ModelId::WhisperLargeV3Turbo {
                (0, &entered_a_tx, &resume_a_rx)
            } else {
                (1, &entered_b_tx, &resume_b_rx)
            };
            if first_observation[index] {
                first_observation[index] = false;
                entered.send(()).unwrap();
                resume.recv_timeout(WAIT).unwrap();
            }
            let mut snapshot = ready_snapshot(catalog.root(), generation, outcome);
            snapshot.entries.retain(|entry| entry.id == id);
            snapshot
        });
    (
        requests,
        catalog,
        admission,
        PausedObservations {
            entered_a,
            resume_a,
            entered_b,
            resume_b,
        },
    )
}

type RecordedEffects = Arc<Mutex<Vec<ImportEffect>>>;
type RecordedEvents = Arc<Mutex<VecDeque<ImportEvent>>>;

fn application_for(
    root: &std::path::Path,
    requester: &CatalogInspectionSender,
    admission: Receiver<CatalogSnapshot>,
) -> (
    DesktopAdmissionApplication<RecordingIo>,
    RecordedEffects,
    RecordedEvents,
) {
    let io = RecordingIo::default();
    let effects = Arc::clone(&io.effects);
    let events = Arc::clone(&io.events);
    let app = guarded_application(io, root, "C:/legacy/turbo.bin".into(), admission)
        .with_inspection_requests(requester.clone());
    (app, effects, events)
}

fn refresh_until_admitted(
    app: &mut DesktopAdmissionApplication<RecordingIo>,
    expected_generation: u64,
) -> AppView {
    let deadline = std::time::Instant::now() + WAIT;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if app.admission_generation >= expected_generation {
            return view;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "admission snapshot timed out: expected generation {expected_generation}, received {}, channel failed: {}, last message: {:?}",
            app.admission_generation,
            app.admission_catalog_failed,
            view.message
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn check_intermediate_publication(resume: bool, outcome: ModelAvailability, invalidate: bool) {
    let root = Root::new();
    let (requests, catalog, admission, paused) = paused_catalog(root.0.clone(), outcome);
    catalog.recv_timeout(WAIT).unwrap();
    let (mut app, effects, events) = application_for(&root.0, &requests, admission);
    let request = request_for(&root.0, 0xe801);
    let _cleanup = ObservationCleanup {
        requests: requests.clone(),
        resume_a: paused.resume_a.clone(),
        resume_b: paused.resume_b.clone(),
    };
    events
        .lock()
        .unwrap()
        .push_back(ImportEvent::Queue(vec![queue_entry(&request, resume)]));
    let before = app.dispatch(AppCommand::Refresh).unwrap();
    requests.select(ModelId::WhisperLargeV3Turbo, 1).unwrap();
    paused.entered_a.recv_timeout(WAIT).unwrap();
    app.dispatch(queued_command(&request, resume)).unwrap();
    paused.resume_a.send(()).unwrap();
    paused.entered_b.recv_timeout(WAIT).unwrap();
    catalog.recv_timeout(WAIT).unwrap();
    let intermediate = app.dispatch(AppCommand::Refresh).unwrap();
    assert!(
        intermediate
            .message
            .as_deref()
            .unwrap()
            .contains("Vérification du modèle en cours")
    );
    assert!(!intermediate.message.as_deref().unwrap().contains("échouée"));
    assert_eq!(intermediate.queue, before.queue);
    assert!(effects.lock().unwrap().is_empty());
    if invalidate {
        fs::write(
            root.path(ModelId::ParakeetTdt06BV3),
            b"changed identity metadata size",
        )
        .unwrap();
    }
    paused.resume_b.send(()).unwrap();
    let final_publication = catalog.recv_timeout(WAIT).unwrap();
    let final_view = refresh_until_admitted(&mut app, final_publication.generation);
    let notice = final_view.message.as_deref().unwrap();
    if outcome == ModelAvailability::Ready && !invalidate {
        assert!(notice.contains("Modèle vérifié"), "{notice}");
        assert!(notice.contains(if resume { "reprise" } else { "Traiter" }));
    } else {
        assert!(
            notice.contains("Vérification du modèle échouée"),
            "{notice}"
        );
        assert!(
            notice.contains(if invalidate { "modifiée" } else { "synthetic" }),
            "{notice}"
        );
    }
    assert_eq!(final_view.queue, before.queue);
    assert_eq!(final_view.active_generation, before.active_generation);
    assert!(effects.lock().unwrap().is_empty());
    requests.close();
}

#[test]
fn final_catalog_publication_waits_until_application_admission_receives_it() {
    let mut harness = Harness::new();
    let request = harness.request();
    harness
        .events
        .lock()
        .unwrap()
        .push_back(ImportEvent::Queue(vec![queue_entry(&request, false)]));
    let before = harness.app.dispatch(AppCommand::Refresh).unwrap();
    harness
        .app
        .dispatch(queued_command(&request, false))
        .unwrap();
    let need = harness.receiver.next(Duration::ZERO).unwrap().unwrap();
    let expected_generation = 2;
    assert!(
        harness
            .receiver
            .publish(Some(need), expected_generation, [1, need.version])
            .unwrap()
    );
    let final_snapshot = ready_snapshot(
        &harness.root.0,
        expected_generation,
        ModelAvailability::Ready,
    );
    let (published_tx, published_rx) = mpsc::channel();
    published_tx.send(final_snapshot.clone()).unwrap();
    let published = published_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(published.generation, expected_generation);

    let waiting = harness.app.dispatch(AppCommand::Refresh).unwrap();
    assert!(
        waiting
            .message
            .as_deref()
            .unwrap()
            .contains("Vérification du modèle en cours")
    );
    assert_eq!(waiting.queue, before.queue);
    assert_eq!(waiting.active_generation, before.active_generation);
    assert!(harness.effects.lock().unwrap().is_empty());

    harness.catalog.send(published).unwrap();
    let admitted = refresh_until_admitted(&mut harness.app, expected_generation);
    let notice = admitted.message.as_deref().unwrap();
    assert!(notice.contains("Modèle vérifié"), "{notice}");
    assert!(notice.contains("Traiter"), "{notice}");
    assert_eq!(admitted.queue, before.queue);
    assert_eq!(admitted.active_generation, before.active_generation);
    assert!(harness.effects.lock().unwrap().is_empty());
    harness.requester.close();
}

#[test]
fn r1_intermediate_a_publication_keeps_b_waiting_until_ready_for_process_and_resume() {
    for resume in [false, true] {
        check_intermediate_publication(resume, ModelAvailability::Ready, false);
    }
}

#[test]
fn r1_only_b_observation_reports_b_failure_or_identity_invalidation() {
    for resume in [false, true] {
        for outcome in [ModelAvailability::Missing, ModelAvailability::Corrupt] {
            check_intermediate_publication(resume, outcome, false);
        }
        check_intermediate_publication(resume, ModelAvailability::Ready, true);
    }
}

type FixtureObserver = Box<dyn FnMut(&LocalModelCatalog, ModelId, u64) -> CatalogSnapshot + Send>;

fn remove_queue_interest(
    app: &mut DesktopAdmissionApplication<RecordingIo>,
    events: &RecordedEvents,
    removed: &ImportRequest,
    remaining: Vec<ImportQueueEntry>,
) {
    app.dispatch(AppCommand::RemoveQueued {
        job_id: removed.job_id,
        generation: removed.generation,
    })
    .unwrap();
    events
        .lock()
        .unwrap()
        .push_back(ImportEvent::Queue(remaining));
    app.dispatch(AppCommand::Refresh).unwrap();
}

fn check_removed_interest_stops_monitoring(resume: bool, another_need: bool) {
    let root = Root::new();
    let (requester, receiver) = catalog_inspection_channel();
    requester.select(ModelId::WhisperLargeV3Turbo, 1).unwrap();
    receiver.next(Duration::ZERO).unwrap().unwrap();
    let (catalog_tx, catalog) = mpsc::channel();
    let (admission_tx, admission) = mpsc::channel();
    let (entered_tx, entered) = mpsc::sync_channel(1);
    let (release, release_rx) = mpsc::sync_channel(1);
    let calls = Arc::new(AtomicU64::new(0));
    let observed_calls = Arc::clone(&calls);
    let inspect: FixtureObserver = Box::new(move |catalog, id, generation| {
        assert_eq!(id, ModelId::ParakeetTdt06BV3);
        if observed_calls.fetch_add(1, Ordering::SeqCst) == 0 {
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
        }
        // Real adapter read, with injected qualification for the small fixture.
        let mut observed = catalog.inspect_model(id, generation);
        let entry = &mut observed.entries[0];
        entry.availability = ModelAvailability::Ready;
        entry.observed_sha256 = Some(entry.expected_sha256.clone());
        entry.observed_size_bytes = Some(entry.expected_size_bytes);
        entry.diagnostic = None;
        observed
    });
    let mut service = CatalogService {
        catalog: LocalModelCatalog::new(&root.0),
        snapshot: ready_snapshot(&root.0, 1, ModelAvailability::Uninspected),
        observed_versions: BTreeMap::from([(ModelId::WhisperLargeV3Turbo, 1)]),
        observed_identities: BTreeMap::from([(
            ModelId::WhisperLargeV3Turbo,
            file_metadata(&root.path(ModelId::WhisperLargeV3Turbo)),
        )]),
        gpu_route_metadata: None,
        receiver,
        outputs: [catalog_tx, admission_tx],
        inspect,
    };
    assert!(service.publish(None));
    catalog.recv_timeout(WAIT).unwrap();
    let (mut app, effects, events) = application_for(&root.0, &requester, admission);
    let first = request_for(&root.0, 0xe811);
    let second = request_for(&root.0, 0xe812);
    let mut queue = vec![queue_entry(&first, resume)];
    if another_need {
        queue.push(queue_entry(&second, resume));
    }
    events.lock().unwrap().push_back(ImportEvent::Queue(queue));
    app.dispatch(AppCommand::Refresh).unwrap();
    app.dispatch(queued_command(&first, resume)).unwrap();
    if another_need {
        app.dispatch(queued_command(&second, resume)).unwrap();
    }
    let need = service.receiver.next(Duration::ZERO).unwrap().unwrap();
    let calculating = std::thread::spawn(move || {
        service.observe(need).unwrap();
        service
    });
    entered.recv_timeout(WAIT).unwrap();
    let removed = if another_need { &second } else { &first };
    let remaining = if another_need {
        vec![queue_entry(&first, resume)]
    } else {
        vec![]
    };
    remove_queue_interest(&mut app, &events, removed, remaining);
    let effects_before_completion = effects.lock().unwrap().clone();
    release.send(()).unwrap();
    let mut service = calculating.join().unwrap();
    catalog.recv_timeout(WAIT).unwrap();
    app.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(app.inspection_interests.len(), usize::from(another_need));
    if another_need {
        assert_eq!(app.visible_inspection.unwrap().job, first.job_id.0);
        assert!(
            app.admission_notice
                .as_deref()
                .unwrap()
                .contains("Modèle vérifié")
        );
    } else {
        assert!(app.admission_notice.is_none());
    }
    fs::write(
        root.path(ModelId::ParakeetTdt06BV3),
        b"new model identity with a different length",
    )
    .unwrap();
    service.refresh_interested().unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        if another_need { 2 } else { 1 }
    );
    assert_eq!(
        model_sha256_full_read_count(&root.path(ModelId::ParakeetTdt06BV3)).unwrap(),
        u64::from(another_need)
    );
    assert_eq!(*effects.lock().unwrap(), effects_before_completion);
    if another_need {
        remove_queue_interest(&mut app, &events, &first, vec![]);
        fs::write(
            root.path(ModelId::ParakeetTdt06BV3),
            b"another changed model identity",
        )
        .unwrap();
        service.refresh_interested().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(app.admission_notice.is_none());
    }
    assert!(
        !service
            .receiver
            .interests()
            .unwrap()
            .iter()
            .any(|need| need.model == ModelId::ParakeetTdt06BV3)
    );
    requester.close();
}

#[test]
fn r2_removed_action_cannot_keep_monitoring_or_reinvite_after_inflight_completion() {
    for resume in [false, true] {
        check_removed_interest_stops_monitoring(resume, false);
    }
}

#[test]
fn r2_removing_one_action_preserves_another_current_need_for_the_same_model() {
    for resume in [false, true] {
        check_removed_interest_stops_monitoring(resume, true);
    }
}
