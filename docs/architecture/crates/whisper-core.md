Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-core

## Rôle et type

Définit commandes, vues et ports et transforme les commandes en effets. `crates/whisper-core/src/application/contracts.rs:1-318` ; `crates/whisper-core/src/application/runtime/commands.rs:1-145` ; `crates/whisper-core/src/ports.rs:1-873` ; `crates/whisper-core/src/compute_policy.rs:1-67` ; `crates/whisper-core/src/parakeet_scheduler.rs:1-672`

Contient les politiques CPU/GPU et l’ordonnanceur Parakeet. `crates/whisper-core/src/application/contracts.rs:1-318` ; `crates/whisper-core/src/application/runtime/commands.rs:1-145` ; `crates/whisper-core/src/ports.rs:1-873` ; `crates/whisper-core/src/compute_policy.rs:1-67` ; `crates/whisper-core/src/parakeet_scheduler.rs:1-672`

Type : **Bibliothèque de domaine et application**. `crates/whisper-core/src/application/contracts.rs:1-318` ; `crates/whisper-core/src/application/runtime/commands.rs:1-145` ; `crates/whisper-core/src/ports.rs:1-873` ; `crates/whisper-core/src/compute_policy.rs:1-67` ; `crates/whisper-core/src/parakeet_scheduler.rs:1-672`

Mesures : **11251 production / 8561 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| serde | externe | dependencies | {"workspace": true} | `crates/whisper-core/Cargo.toml:9` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-core/Cargo.toml:1-12`.

## Interfaces publiques et usages intercrates

| Consommateur | Contexte | Import observé | Source |
| --- | --- | --- | --- |
| whisper-adapters | tests | `use whisper_core::ports::ParakeetDocumentPort;` | `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:512` |
| whisper-adapters | tests | `use whisper_core::SegmentId;` | `crates/whisper-adapters/src/archive/tests/recovery.rs:543` |
| whisper-adapters | tests | `use whisper_core::SegmentId;` | `crates/whisper-adapters/src/archive/tests/recovery.rs:725` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, JobId, SourceRange, ports::{ DuplicateImportChoice, ImportRequest, LiveRequest, PublishedImport, ReplacementCleanupInfo, TreatmentIdentity, WorkerSegment, }, };` | `crates/whisper-adapters/src/archive.rs:10-16` |
| whisper-adapters | hors tests / cfg | `use whisper_core::microphone::{MicrophoneChoice, MicrophoneDevice, MicrophonePort};` | `crates/whisper-adapters/src/capture.rs:11` |
| whisper-adapters | hors tests / cfg | `use whisper_core::microphone_level::{GainDb, GainRevision};` | `crates/whisper-adapters/src/capture.rs:12` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ ipc::{WorkerCommand, WorkerEvent}, ports::{DecodeRequest, DecodedPcmBlock, DecoderPort, PortError}, };` | `crates/whisper-adapters/src/decoder.rs:3-6` |
| whisper-adapters | tests | `use whisper_core::{ Generation, JobId, SourceRange, ipc::WorkerEvent, ports::{DecodedPcmBlock, PcmBlock, SourceIdentity}, };` | `crates/whisper-adapters/src/decoder.rs:107-111` |
| whisper-adapters | tests | `use whisper_core::{ ComputeChoice, Generation, ImportEvent, JobConfig, JobId, LanguageChoice, LiveRequest, microphone_level::GainDb, model_catalog::ModelId, };` | `crates/whisper-adapters/src/direct_live/d2_corpus.rs:13-16` |
| whisper-adapters | tests | `use whisper_core::ports::WorkerSegment;` | `crates/whisper-adapters/src/direct_live/tests.rs:11` |
| whisper-adapters | tests | `use whisper_core::{ComputeChoice, JobConfig, LanguageChoice, SegmentId, SourceRange};` | `crates/whisper-adapters/src/direct_live/tests.rs:12` |
| whisper-adapters | tests | `use whisper_core::ipc::{BackendKind, WorkerEvent, WorkerSegmentDto};` | `crates/whisper-adapters/src/direct_live/tests.rs:1104` |
| whisper-adapters | tests | `use whisper_core::ipc::{BackendKind, WorkerErrorCode, WorkerEvent};` | `crates/whisper-adapters/src/direct_live/tests.rs:1174` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ ComputeChoice, Generation, ImportEvent, JobId, LiveRequest, SegmentId, SourceRange, ipc::{BackendKind, WorkerCommand, WorkerErrorCode, WorkerEvent}, ports::WorkerSegment, };` | `crates/whisper-adapters/src/direct_live.rs:36-40` |
| whisper-adapters | hors tests / cfg | `use whisper_core::gpu_inventory::{ GpuAvailability, GpuCause, GpuIdentityMatch, GpuQualifiedObservation, GpuSupport, };` | `crates/whisper-adapters/src/gpu_inventory.rs:7-9` |
| whisper-adapters | hors tests / cfg | `use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};` | `crates/whisper-adapters/src/gpu_inventory.rs:10` |
| whisper-adapters | hors tests / cfg | `use whisper_core::gpu_inventory_app::{GpuCollection, GpuInventoryPort, GpuQualifiedCollection};` | `crates/whisper-adapters/src/gpu_inventory.rs:11` |
| whisper-adapters | tests | `use whisper_core::gpu_inventory::{ GpuAvailability, GpuCatalogueCoverage, GpuCause, GpuIdentityMatch, GpuObservation, GpuPresence, GpuState, GpuSupport, qualify, };` | `crates/whisper-adapters/src/gpu_inventory.rs:621-624` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, JobId, microphone_level::{ GainRevision, LevelMeasurement, MeterLevel, MicrophoneMeter, MicrophoneSpectrum, MicrophoneSpectrumUpdate, SpectrumBin, monotonic_millis, monotonic_millis_at, }, };` | `crates/whisper-adapters/src/microphone_probe.rs:13-19` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{model_catalog::ModelAvailability, model_registry::MODELS};` | `crates/whisper-adapters/src/model_catalog.rs:14` |
| whisper-adapters | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-adapters/src/model_catalog.rs:290` |
| whisper-adapters | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-adapters/src/model_catalog.rs:310` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{model_catalog::ModelId, model_registry::model};` | `crates/whisper-adapters/src/model_catalog.rs:331` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{model_catalog::ModelId, model_selection_app::is_known_historical_path};` | `crates/whisper-adapters/src/model_catalog.rs:365` |
| whisper-adapters | hors tests / cfg | `use whisper_core::SourceRange;` | `crates/whisper-adapters/src/parakeet_confirmation.rs:7` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, JobId, parakeet_windows::{WINDOW_MAX_SAMPLES, WINDOW_MIN_SAMPLES}, ports::{ ParakeetDocumentEntry, ParakeetDocumentPage, ParakeetDocumentPort, ParakeetPreview, ParakeetWindowCoverage, PortError, WorkerSegment, }, };` | `crates/whisper-adapters/src/parakeet_document.rs:8-15` |
| whisper-adapters | hors tests / cfg | `use whisper_core::parakeet_ipc::{ Command as WorkerCommand, Envelope, Event, PARAKEET_IPC_VERSION, PARAKEET_MAX_FRAME_BYTES, PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity, };` | `crates/whisper-adapters/src/parakeet_import.rs:17-20` |
| whisper-adapters | hors tests / cfg | `use whisper_core::parakeet_windows::{PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE};` | `crates/whisper-adapters/src/parakeet_import.rs:21` |
| whisper-adapters | hors tests / cfg | `use whisper_core::model_registry::{ PARAKEET_TEST_GPU_PCI_BUS_ID, PARAKEET_TEST_GPU_PNP_INSTANCE_ID, PARAKEET_TEST_GPU_UUID, };` | `crates/whisper-adapters/src/parakeet_import.rs:781-783` |
| whisper-adapters | tests | `use whisper_core::microphone_level::{MicrophoneSpectrumUpdate, SpectrumBin};` | `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:8` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ ComputeChoice, Generation, ImportEvent, JobId, LiveRequest, SegmentId, SourceRange, parakeet_ipc::{Event as WorkerEvent, Word}, parakeet_windows::{AbsoluteWordSamples, Window, absolute_word_samples}, };` | `crates/whisper-adapters/src/parakeet_live.rs:32-36` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, JobId, SourceRange, parakeet_session::{AudioWindow, SessionIdentity, TranscriptionStrategy, WindowCursor}, parakeet_windows::{MARGIN_SAMPLES, WINDOW_MAX_SAMPLES}, ports::{ParakeetDocumentPort, ParakeetWindowCoverage, PortError, WorkerSegment}, };` | `crates/whisper-adapters/src/parakeet_pipeline.rs:3-8` |
| whisper-adapters | hors tests / cfg | `use whisper_core::path_settings::{PathField, PathPort, SaveAcknowledgement, StoredPaths};` | `crates/whisper-adapters/src/path_settings.rs:9` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ model_catalog::ModelId, model_selection_app::is_known_historical_path, path_settings::{ LegacyModelPath, ModelPreferenceAcknowledgement, ModelPreferencePort, ModelPreferenceSnapshot, StartupComputePreference, }, };` | `crates/whisper-adapters/src/path_settings.rs:10-17` |
| whisper-adapters | tests | `use whisper_core::path_settings::{PathDefaults, PathPort, PathSettingsApp};` | `crates/whisper-adapters/src/path_settings.rs:493` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ComputeChoice, Generation, JobId, ports::ImportRequest};` | `crates/whisper-adapters/src/queue_store.rs:9` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};` | `crates/whisper-adapters/src/queue_store.rs:27` |
| whisper-adapters | tests | `use whisper_core::{JobConfig, ports::ImportRequest};` | `crates/whisper-adapters/src/queue_store.rs:564` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};` | `crates/whisper-adapters/src/recovery.rs:6` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ports::SourceSnapshotRecoveryCandidate;` | `crates/whisper-adapters/src/source_binding.rs:11` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{Generation, JobId, ports::ImportRequest};` | `crates/whisper-adapters/src/source_binding.rs:12` |
| whisper-adapters | tests | `use whisper_core::ports::ImportRequest;` | `crates/whisper-adapters/src/source_binding.rs:1523` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{BackendAttempt, ComputeChoice, ComputePolicy, ComputePolicyError};` | `crates/whisper-adapters/src/supervisor.rs:14` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ ComputeChoice, ipc::{WorkerCommand, WorkerEvent}, ports::{ImportRequest, LiveRequest}, };` | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:8-12` |
| whisper-adapters | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:123` |
| whisper-adapters | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:237` |
| whisper-adapters | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:272` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ComputeChoice, ports::ImportRequest};` | `crates/whisper-adapters/src/worker_ipc/backends/native.rs:3` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, JobId, ipc::{BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerEvent}, ports::PortError, };` | `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:23-27` |
| whisper-adapters | hors tests / cfg | `use whisper_core::Generation;` | `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:15` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, JobId, ports::{ImportRequest, WorkerSegment}, };` | `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:3-6` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ports::ParakeetDocumentPort;` | `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:32` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ComputeChoice;` | `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:12` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ports::PortError;` | `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:2` |
| whisper-adapters | hors tests / cfg | `use whisper_core::ports::ParakeetDocumentPort;` | `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:322` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ImportEvent, JobId};` | `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:9` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ Generation, ImportEffect, ImportEvent, ImportIoPort, JobId, ports::{PortError, WorkerSegment}, };` | `crates/whisper-adapters/src/worker_ipc/service/mod.rs:20-23` |
| whisper-adapters | tests | `use whisper_core::gpu_inventory_app::GpuInventoryPort;` | `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:779` |
| whisper-adapters | tests | `use whisper_core::ports::DuplicateImportChoice;` | `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:121` |
| whisper-adapters | tests | `use whisper_core::parakeet_scheduler::{ScheduledCall, ScheduledWork, SchedulerSession};` | `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2476` |
| whisper-adapters | tests | `use whisper_core::ports::ParakeetDocumentPort;` | `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:5` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, JobState, LanguageChoice, };` | `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:6-9` |
| whisper-adapters | tests | `use whisper_core::ImportIoPort;` | `crates/whisper-adapters/src/worker_ipc.rs:109` |
| whisper-adapters | tests | `use whisper_core::ipc::IpcEnvelope;` | `crates/whisper-adapters/src/worker_ipc.rs:111` |
| whisper-adapters | hors tests / cfg | `use whisper_core::{ ArchiveHistoryItem, ComputeChoice, DiagnosticNotice, Generation, ImportEffect, ImportEvent, JobId, ipc::{BackendKind, WorkerCommand, WorkerEvent}, ports::{ DecodeRequest, DecoderPort, DuplicateImportChoice, DuplicateImportPrompt, ImportRequest, LiveRequest, PortError, PublishedImport, SourceIdentity, TreatmentIdentity, WorkerSegment, }, };` | `crates/whisper-adapters/src/worker_ipc.rs:112-120` |
| whisper-adapters | tests | `use whisper_core::{ domain::{Generation, JobId, SourceRange}, ipc::{WorkerCommand, WorkerEvent}, ports::{DecodeRequest, DecodedPcmBlock, DecoderPort, PcmBlock, PortError, SourceIdentity}, };` | `crates/whisper-adapters/tests/decoder_proxy.rs:2-6` |
| whisper-adapters | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, ports::{ImportQueueStatus, ImportRequest}, };` | `crates/whisper-adapters/tests/durability.rs:6-9` |
| whisper-adapters | tests | `use whisper_core::gpu_inventory::{GpuPresence, GpuState};` | `crates/whisper-adapters/tests/gpu_inventory.rs:2` |
| whisper-adapters | tests | `use whisper_core::ipc::{GpuQualificationResult, WorkerEvent};` | `crates/whisper-adapters/tests/gpu_qualification_transport.rs:1` |
| whisper-adapters | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, gpu_inventory::GpuRuntimeIdentity, ipc::{BackendKind, IpcEnvelope, WorkerCommand, WorkerEvent}, };` | `crates/whisper-adapters/tests/gpu_selection_worker.rs:1-5` |
| whisper-adapters | tests | `use whisper_core::gpu_inventory_app::GpuInventoryApp;` | `crates/whisper-adapters/tests/gpu_selection_worker.rs:89` |
| whisper-adapters | tests | `use whisper_core::{ ComputeChoice, MicrophoneChoice, gpu_inventory::{ GpuAvailability, GpuIdentityMatch, GpuObservation, GpuPresence, GpuQualifiedObservation, GpuRankSource, GpuState, GpuSupport, GpuWindowsRank, qualify, }, gpu_inventory_app::GpuQualifiedInventoryState, gpu_selection::{SelectionResolution, SessionCandidate, resolve}, path_settings::{PathDefaults, PathField, PathSettingsApp, SaveStatus, StoredPaths}, };` | `crates/whisper-adapters/tests/gpu_settings.rs:8-17` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, JobState, LanguageChoice, SourceRange, ipc::{WorkerCommand, WorkerEvent}, ports::{DecodeRequest, DecoderPort, DuplicateImportChoice, PortError, SourceIdentity}, };` | `crates/whisper-adapters/tests/import_cpu.rs:15-20` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, JobState, LanguageChoice, SourceRange, ports::WorkerSegment, };` | `crates/whisper-adapters/tests/import_mp3.rs:6-9` |
| whisper-adapters | tests | `use whisper_core::{ domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice}, model_catalog::ModelId, ports::ImportRequest, };` | `crates/whisper-adapters/tests/legacy_model_requests.rs:8-12` |
| whisper-adapters | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, LiveRequest, SegmentId, SourceRange, ports::WorkerSegment, };` | `crates/whisper-adapters/tests/live_archive.rs:3-6` |
| whisper-adapters | tests | `use whisper_core::microphone_level::{GainDb, MAX_GAIN_DB, MIN_GAIN_DB};` | `crates/whisper-adapters/tests/microphone_level.rs:2` |
| whisper-adapters | tests | `use whisper_core::{ GainDb, Generation, JobId, microphone_level::{GainRevision, MicrophoneSpectrumUpdate}, };` | `crates/whisper-adapters/tests/microphone_probe.rs:12-15` |
| whisper-adapters | tests | `use whisper_core::path_settings::StoredPaths;` | `crates/whisper-adapters/tests/microphone_settings.rs:1` |
| whisper-adapters | tests | `use whisper_core::{ domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice}, model_catalog::{ModelAvailability, ModelId}, model_registry::MODELS, ports::ImportRequest, };` | `crates/whisper-adapters/tests/model_catalog.rs:4-9` |
| whisper-adapters | tests | `use whisper_core::{ model_catalog::ModelId, path_settings::{LegacyModelPath, ModelPreferencePort, PathPort, StartupComputePreference}, };` | `crates/whisper-adapters/tests/model_preferences.rs:4-7` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, JobConfig, JobId, LanguageChoice, SegmentId, SourceRange, model_catalog::ModelId, model_registry::model, ports::{ImportRequest, WorkerSegment}, };` | `crates/whisper-adapters/tests/parakeet_archive.rs:16-22` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, LanguageChoice, model_catalog::{ModelAvailability, ModelId}, model_registry, model_selection_app::ModelCommandGuard, parakeet_ipc::{Event, WorkerIdentity}, parakeet_windows::{ MAX_NATIVE_SAMPLES, PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE, Window, absolute_word_samples, plan_windows, word_belongs_to_central_range, }, };` | `crates/whisper-adapters/tests/parakeet_archive.rs:1457-1468` |
| whisper-adapters | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, model_catalog::{Backend, ModelId}, model_registry::{self, model}, };` | `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:4-8` |
| whisper-adapters | tests | `use whisper_core::parakeet_ipc::{Command, Envelope, ExpectedGpu, WorkerIdentity};` | `crates/whisper-adapters/tests/parakeet_command_wire.rs:2` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, LanguageChoice, model_catalog::ModelId, };` | `crates/whisper-adapters/tests/parakeet_control.rs:15-18` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, LanguageChoice, model_catalog::ModelId, };` | `crates/whisper-adapters/tests/parakeet_import.rs:18-21` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, LanguageChoice, model_catalog::{ModelAvailability, ModelId}, model_registry, model_selection_app::ModelCommandGuard, parakeet_ipc::{Event, WorkerIdentity}, parakeet_windows::{ MAX_NATIVE_SAMPLES, PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE, Window, absolute_word_samples, admissible_silences, plan_windows, word_belongs_to_central_range, }, };` | `crates/whisper-adapters/tests/parakeet_long.rs:11-22` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, JobState, LanguageChoice, model_catalog::ModelId, model_registry::{PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID, model}, };` | `crates/whisper-adapters/tests/parakeet_unified_native.rs:13-18` |
| whisper-adapters | tests | `use whisper_core::gpu_inventory_app::GpuInventoryPort;` | `crates/whisper-adapters/tests/parakeet_unified_native.rs:825` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest, JobConfig, JobId, LanguageChoice, model_catalog::ModelId, ports::DuplicateImportChoice, };` | `crates/whisper-adapters/tests/reprocessing_baseline.rs:13-16` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, JobConfig, JobId, JobState, LanguageChoice, ports::DuplicateImportChoice, ports::ImportRequest, };` | `crates/whisper-adapters/tests/reprocessing_cleanup.rs:16-19` |
| whisper-adapters | tests | `use whisper_core::ports::DuplicateImportChoice;` | `crates/whisper-adapters/tests/reprocessing_create.rs:16` |
| whisper-adapters | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportApplication, JobConfig, JobId, JobState, ports::ImportRequest, };` | `crates/whisper-adapters/tests/reprocessing_create.rs:17-20` |
| whisper-adapters | tests | `use whisper_core::{Generation, JobConfig, JobId, ports::ImportRequest};` | `crates/whisper-adapters/tests/source_binding.rs:10` |
| whisper-adapters | tests | `use whisper_core::{BackendAttempt, ComputeChoice, ComputePolicyError};` | `crates/whisper-adapters/tests/worker_control.rs:9` |
| whisper-bootstrap | hors tests / cfg | `use whisper_core::{AppView, ipc::IPC_PROTOCOL_VERSION};` | `crates/whisper-bootstrap/src/main.rs:3` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory_app::GpuInventoryApp;` | `crates/whisper-desktop/src/root.rs:15` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_selection_app::ModelCommandGuard;` | `crates/whisper-desktop/src/root.rs:16` |
| whisper-desktop | hors tests / cfg | `use whisper_core::path_settings::{ ModelPreferenceSaveStatus, ModelPreferencesApp, StartupComputePreference, };` | `crates/whisper-desktop/src/root.rs:17-19` |
| whisper-desktop | hors tests / cfg | `use whisper_core::path_settings::{PathDefaults, PathSettingsApp};` | `crates/whisper-desktop/src/root.rs:20` |
| whisper-desktop | hors tests / cfg | `use whisper_core::{ AppCommand, AppView, Application, ApplicationError, ComputeChoice, ImportApplication, ImportIoPort, model_catalog::{ModelAvailability, ModelId}, model_registry, };` | `crates/whisper-desktop/src/root.rs:21-26` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_catalog::{Backend, ModelId};` | `crates/whisper-desktop/src/root.rs:408` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-desktop/src/root.rs:517` |
| whisper-desktop | tests | `use whisper_core::ports::{ImportEvent, PortError};` | `crates/whisper-desktop/src/root.rs:784` |
| whisper-desktop | tests | `use whisper_core::{ AppCommand, Application, ComputeChoice, Generation, ImportEffect, ImportRequest, JobConfig, JobId, LanguageChoice, };` | `crates/whisper-desktop/src/root.rs:785-788` |
| whisper-desktop | hors tests / cfg | `use whisper_core::microphone_level::SpectrumBin;` | `crates/whisper-desktop/src/ui/model/view/microphone.rs:9` |
| whisper-desktop | hors tests / cfg | `use whisper_core::engine_capabilities::EngineProjection;` | `crates/whisper-desktop/src/ui/model.rs:10` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory_app::{GpuInventoryApp, GpuInventoryState};` | `crates/whisper-desktop/src/ui/model.rs:11` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_selection::{SelectionResolution, SessionCandidate};` | `crates/whisper-desktop/src/ui/model.rs:12` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_catalog::ModelId;` | `crates/whisper-desktop/src/ui/model.rs:13` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_selection_app::ModelSelectionApp;` | `crates/whisper-desktop/src/ui/model.rs:14` |
| whisper-desktop | hors tests / cfg | `use whisper_core::path_settings::{PathDefaults, PathField, PathSettingsApp};` | `crates/whisper-desktop/src/ui/model.rs:15` |
| whisper-desktop | hors tests / cfg | `use whisper_core::ports::DuplicateImportChoice;` | `crates/whisper-desktop/src/ui/model.rs:16` |
| whisper-desktop | hors tests / cfg | `use whisper_core::{ AppCommand, AppFacade, AppView, Application, ApplicationError, ComputeChoice, Generation, ImportRequest, JobConfig, JobId, JobState, LiveRequest, MicrophoneChoice, MicrophonePort, MicrophoneSettings, NullMicrophonePort, };` | `crates/whisper-desktop/src/ui/model.rs:17-21` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory::GpuCatalogueCoverage;` | `crates/whisper-desktop/src/ui/presentation/gpu.rs:15` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory::{GpuAvailability, GpuIdentityMatch, GpuState, GpuSupport};` | `crates/whisper-desktop/src/ui/presentation/gpu.rs:130` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory::{ GpuAvailability as Availability, GpuIdentityMatch, GpuState, GpuSupport as Support, };` | `crates/whisper-desktop/src/ui/presentation/gpu.rs:180-182` |
| whisper-desktop | hors tests / cfg | `use whisper_core::engine_capabilities::{LanguageOption, RefusalCause};` | `crates/whisper-desktop/src/ui/presentation/language.rs:16` |
| whisper-desktop | hors tests / cfg | `use whisper_core::microphone_level::MeasurementStatus;` | `crates/whisper-desktop/src/ui/presentation/microphone.rs:49` |
| whisper-desktop | hors tests / cfg | `use whisper_core::ports::UnifiedSessionPhase as Phase;` | `crates/whisper-desktop/src/ui/presentation/selection.rs:94` |
| whisper-desktop | hors tests / cfg | `use whisper_core::engine_capabilities::EngineProjection;` | `crates/whisper-desktop/src/ui/presentation.rs:4` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory::GpuQualifiedDevice;` | `crates/whisper-desktop/src/ui/presentation.rs:5` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_inventory_app::{GpuInventoryState, GpuQualifiedInventoryState};` | `crates/whisper-desktop/src/ui/presentation.rs:6` |
| whisper-desktop | hors tests / cfg | `use whisper_core::gpu_selection::{SelectionResolution, SessionCandidate};` | `crates/whisper-desktop/src/ui/presentation.rs:7` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_catalog::{Backend, ModelId};` | `crates/whisper-desktop/src/ui/presentation.rs:8` |
| whisper-desktop | hors tests / cfg | `use whisper_core::model_registry::TURBO_LANGUAGE_INPUTS;` | `crates/whisper-desktop/src/ui/presentation.rs:9` |
| whisper-desktop | hors tests / cfg | `use whisper_core::path_settings::SaveStatus;` | `crates/whisper-desktop/src/ui/presentation.rs:10` |
| whisper-desktop | hors tests / cfg | `use whisper_core::{ AppCommand, AppView, ComputeChoice, Generation, JobId, JobState, LanguageChoice, MicrophoneChoice, MicrophoneDevice, };` | `crates/whisper-desktop/src/ui/presentation.rs:11-14` |
| whisper-desktop | tests | `use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};` | `crates/whisper-desktop/src/ui/tests/support.rs:19` |
| whisper-desktop | tests | `use whisper_core::gpu_inventory_app::{ GpuCollection, GpuInventoryApp, GpuInventoryPort, GpuInventoryState, };` | `crates/whisper-desktop/src/ui/tests/support.rs:20-22` |
| whisper-desktop | tests | `use whisper_core::{ AppCommand, AppView, Application, ApplicationError, ComputeChoice, Generation, ImportEffect, ImportRequest, JobConfig, JobId, LanguageChoice, MicrophoneChoice, MicrophoneDevice, MicrophonePort, };` | `crates/whisper-desktop/src/ui/tests/support.rs:23-27` |
| whisper-desktop | tests | `use whisper_core::ports::ImportEvent;` | `crates/whisper-desktop/src/ui/tests/support.rs:220` |
| whisper-desktop | tests | `use whisper_core::microphone_level::GainRevision;` | `crates/whisper-desktop/src/ui/tests/support.rs:347` |
| whisper-desktop | tests | `use whisper_core::ports::{UnifiedParakeetSessionView, UnifiedSessionPhase as Phase};` | `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:86` |
| whisper-desktop | tests | `use whisper_core::ports::{UnifiedParakeetSessionView, UnifiedSessionPhase as Phase};` | `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:176` |
| whisper-desktop | tests | `use whisper_core::{ ImportApplication, JobState, application::{DirectCapturePhase, DirectEnginePhase}, ports::{ImportEvent, ImportIoPort, PortError}, };` | `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:369-373` |
| whisper-desktop | tests | `use whisper_core::{ JobState, application::{DirectCapturePhase as Capture, DirectEnginePhase as Engine}, };` | `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:516-519` |
| whisper-desktop | tests | `use whisper_core::ports::{ UnifiedAudioPartPage, UnifiedParakeetSelectionView, UnifiedParakeetSessionView, UnifiedSessionPhase, };` | `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:562-565` |
| whisper-desktop | tests | `use whisper_core::ports::{ UnifiedAudioPart, UnifiedAudioPartPage, UnifiedParakeetSelectionView, UnifiedParakeetSessionView, UnifiedSessionPhase, };` | `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:692-695` |
| whisper-desktop | tests | `use whisper_core::gpu_inventory::{ Coverage, GpuAvailability, GpuCatalogueCoverage, GpuDevice, GpuIdentityMatch, GpuInventory, GpuQualifiedCatalogue, GpuQualifiedDevice, GpuRankSource, GpuState, GpuSupport, };` | `crates/whisper-desktop/tests/gpu_inventory_ui.rs:2-5` |
| whisper-desktop | tests | `use whisper_core::gpu_inventory_app::{GpuInventoryState, GpuQualifiedInventoryState};` | `crates/whisper-desktop/tests/gpu_inventory_ui.rs:6` |
| whisper-desktop | tests | `use whisper_core::{ gpu_inventory::{ GpuAvailability, GpuDevice, GpuIdentityMatch, GpuQualifiedDevice, GpuState, GpuSupport, }, gpu_selection::{GpuCandidate, SelectionReason, SelectionResolution, SessionCandidate}, path_settings::SaveStatus, };` | `crates/whisper-desktop/tests/gpu_selection_ui.rs:2-8` |
| whisper-desktop | tests | `use whisper_core::{Generation, JobId, MicrophoneDevice, microphone_level::MeterLevel};` | `crates/whisper-desktop/tests/microphone_level_ui.rs:1` |
| whisper-desktop | tests | `use whisper_core::{ ComputeChoice, engine_capabilities::ResourceObservation, model_catalog::{Backend, ModelAvailability, ModelId}, model_registry, path_settings::StartupComputePreference, };` | `crates/whisper-desktop/tests/model_selection_ui.rs:2-8` |
| whisper-desktop | tests | `use whisper_core::engine_capabilities::RefusalCause;` | `crates/whisper-desktop/tests/model_selection_ui.rs:108` |
| whisper-desktop | tests | `use whisper_core::{Generation, JobId, ports::SourceSnapshotRecoveryCandidate};` | `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:2` |
| whisper-worker-common | tests | `use whisper_core::{ Generation, JobId, SourceRange, ports::{DecodeRequest, SourceIdentity}, };` | `crates/whisper-worker-common/src/decoder/tests.rs:12-15` |
| whisper-worker-common | hors tests / cfg | `use whisper_core::ports::DecodeRequest;` | `crates/whisper-worker-common/src/decoder.rs:11` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_core::ports::DecodeRequest;` | `crates/whisper-worker-cpu/src/decoder.rs:2` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_core::ipc::{IpcEnvelope, WorkerErrorCode, WorkerEvent};` | `crates/whisper-worker-cpu/src/diagnostics.rs:3` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_core::{ Generation, JobConfig, JobId, SegmentId, domain::{ComputeChoice, SourceRange}, ipc::{ BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent, WorkerSegmentDto, }, ports::{DecodeRequest, DecodedPcmBlock, PcmBlock, WorkerSegment}, };` | `crates/whisper-worker-cpu/src/session/mod.rs:19-27` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent};` | `crates/whisper-worker-cpu/src/startup.rs:15` |
| whisper-worker-cpu | tests | `use whisper_core::ipc::IpcEnvelope;` | `crates/whisper-worker-cpu/src/worker/tests.rs:3` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_core::ipc::{IPC_PROTOCOL_VERSION, WorkerCommand};` | `crates/whisper-worker-cpu/src/worker.rs:3` |
| whisper-worker-cpu | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent}, };` | `crates/whisper-worker-cpu/tests/live_worker.rs:5-8` |
| whisper-worker-cpu | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, ipc::{BackendKind, IpcEnvelope, WorkerCommand, WorkerEvent}, };` | `crates/whisper-worker-cpu/tests/reusable_imports.rs:15-18` |
| whisper-worker-cpu | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, SourceRange, ipc::{IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent}, };` | `crates/whisper-worker-cpu/tests/worker_contract.rs:10-13` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::ports::DecodeRequest;` | `crates/whisper-worker-gpu/src/decoder.rs:2` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::ipc::{IpcEnvelope, WorkerErrorCode, WorkerEvent};` | `crates/whisper-worker-gpu/src/diagnostics.rs:3` |
| whisper-worker-gpu | tests | `use whisper_core::gpu_inventory::GpuRuntimeIdentity;` | `crates/whisper-worker-gpu/src/engine_cache/tests.rs:2` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::gpu_inventory::GpuRuntimeIdentity;` | `crates/whisper-worker-gpu/src/gpu_attestation.rs:1` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent};` | `crates/whisper-worker-gpu/src/gpu_commands.rs:5` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::ipc::{GpuQualificationResult, GpuQualificationTarget};` | `crates/whisper-worker-gpu/src/gpu_qualification.rs:1` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::{ Generation, JobConfig, JobId, SegmentId, domain::{ComputeChoice, SourceRange}, gpu_inventory::GpuRuntimeIdentity, ipc::{ BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent, WorkerSegmentDto, }, ports::{DecodeRequest, DecodedPcmBlock, PcmBlock, WorkerSegment}, };` | `crates/whisper-worker-gpu/src/session/mod.rs:20-29` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent};` | `crates/whisper-worker-gpu/src/startup.rs:12` |
| whisper-worker-gpu | tests | `use whisper_core::ipc::IpcEnvelope;` | `crates/whisper-worker-gpu/src/worker/tests.rs:3` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_core::ipc::{IPC_PROTOCOL_VERSION, WorkerCommand};` | `crates/whisper-worker-gpu/src/worker.rs:3` |
| whisper-worker-gpu | tests | `use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerEvent};` | `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:5` |
| whisper-worker-gpu | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, SourceRange, ipc::{IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent}, };` | `crates/whisper-worker-gpu/tests/worker_contract.rs:10-13` |
| whisper-worker-gpu | tests | `use whisper_core::gpu_inventory::GpuRuntimeIdentity;` | `crates/whisper-worker-gpu/tests/worker_contract.rs:67` |
| whisper-worker-gpu | tests | `use whisper_core::{ ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, ipc::{BackendKind, IpcEnvelope, WorkerCommand, WorkerEvent}, };` | `crates/whisper-worker-gpu/tests/worker_contract.rs:300-303` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;` | `crates/whisper-worker-parakeet-cpu/src/args.rs:2` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_windows::{PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE};` | `crates/whisper-worker-parakeet-cpu/src/args.rs:3` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;` | `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:7` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::{ExpectedGpu, GpuEvidence};` | `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:6` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::{Envelope, Event, PARAKEET_MAX_FRAME_BYTES, WorkerIdentity};` | `crates/whisper-worker-parakeet-cpu/src/ipc.rs:2` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::Word;` | `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:3` |
| whisper-worker-parakeet-cpu | tests | `use whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE;` | `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:12` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;` | `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:5` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::{ Command, Envelope, Event, ExpectedGpu, PARAKEET_IPC_VERSION, WorkerIdentity, };` | `crates/whisper-worker-parakeet-cpu/src/session.rs:9-11` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_ipc::{Event, WorkerIdentity};` | `crates/whisper-worker-parakeet-cpu/src/startup.rs:12` |
| whisper-worker-parakeet-cpu | hors tests / cfg | `use whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE;` | `crates/whisper-worker-parakeet-cpu/src/transcription.rs:9` |
| whisper-worker-parakeet-cpu | tests | `use whisper_core::{ Generation, JobId, SourceRange, ipc::{IpcEnvelope, WorkerCommand, WorkerEvent}, parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES, ports::{DecodeRequest, SourceIdentity}, };` | `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:11-16` |
| whisper-worker-parakeet-cpu | tests | `use whisper_core::parakeet_ipc::{ Command as WorkerCommand, Envelope, Event, ExpectedGpu, GpuEvidence, PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity, };` | `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:11-14` |
| whisper-worker-parakeet-cpu | tests | `use whisper_core::parakeet_ipc::{ Command as WorkerCommand, Envelope, Event, ExpectedGpu, GpuEvidence, PARAKEET_IPC_VERSION, PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity, };` | `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:16-19` |
| whisper-worker-parakeet-cpu | tests | `use whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE;` | `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:20` |

Interfaces principales avec définitions et sites d’usage (pour desktop, entre cibles du même package) :

| Interface | Définition | Usage observé |
| --- | --- | --- |
| `Application` | `crates/whisper-core/src/application/contracts.rs:277-290` | `crates/whisper-desktop/src/ui/model/controller.rs:142-201` |
| `AppCommand` | `crates/whisper-core/src/application/contracts.rs:13-118` | `crates/whisper-desktop/src/ui/model/controller/actions.rs:61-101` |
| `AppView` | `crates/whisper-core/src/application/contracts.rs:198-251` | `crates/whisper-desktop/src/ui/model.rs:189-276` |
| `ImportIoPort` | `crates/whisper-core/src/ports.rs:870-873` | `crates/whisper-adapters/src/worker_ipc/service/mod.rs:411-596` |
| `ImportEffect` | `crates/whisper-core/src/ports.rs:486-589` | `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:4-236` |
| `ImportEvent` | `crates/whisper-core/src/ports.rs:592-838` | `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:4-236` |
| `DecodeRequest` | `crates/whisper-core/src/ports.rs:41-47` | `crates/whisper-worker-common/src/decoder.rs:16-62` |
| `ParakeetScheduler` | `crates/whisper-core/src/parakeet_scheduler.rs:57-67` | `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:382-1373` |

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-core/src/lib.rs:1-45`. Imports textuels, pas un graphe de symboles résolu.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Processus | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Threads/canaux | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Entrées/sorties standard | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| GPU/natif | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate::application::contracts | Définit AppCommand, AppView, Application, AppFacade et les phases capture/moteur. `crates/whisper-core/src/application/contracts.rs:1-318` | 318 | 0 |
| crate::application::event_metadata | Associe les variantes ImportEvent à un nom et une identité job/génération. `crates/whisper-core/src/application/event_metadata.rs:1-170` | 170 | 0 |
| crate::application::runtime::automatic_final | Suit l’import final automatique et sa persistance jusqu’à publication. `crates/whisper-core/src/application/runtime/automatic_final.rs:1-381` | 381 | 0 |
| crate::application::runtime::closing | Pilote la demande de fermeture, les retraits de file et les arrêts actifs. `crates/whisper-core/src/application/runtime/closing.rs:1-184` | 184 | 0 |
| crate::application::runtime::commands | Vérifie l’admission des modèles et route les commandes Application. `crates/whisper-core/src/application/runtime/commands.rs:1-145` | 145 | 0 |
| crate::application::runtime::events | Route les événements import, micro, capture et finalisation. `crates/whisper-core/src/application/runtime/events.rs:1-130` | 130 | 0 |
| crate::application::runtime::import | Gère préparation, segments, acquittements, publication et reprise import. `crates/whisper-core/src/application/runtime/import.rs:1-515` | 515 | 0 |
| crate::application::runtime::live | Traite commandes/événements live, compteurs audio et finalisation CPU. `crates/whisper-core/src/application/runtime/live.rs:1-720` | 720 | 0 |
| crate::application::runtime::microphone | Gère commandes/événements de test microphone et de gain. `crates/whisper-core/src/application/runtime/microphone.rs:1-370` | 370 | 0 |
| crate::application::runtime | Porte l’état ImportApplication et construit files, compteurs et vues. `crates/whisper-core/src/application/runtime/mod.rs:1-150` | 140 | 10 |
| crate::application::runtime::parakeet_views | Actualise les cartes de captures Parakeet et leurs textes/états. `crates/whisper-core/src/application/runtime/parakeet_views.rs:1-572` | 572 | 0 |
| crate::application::runtime::poll | Draine les événements du port et les segments en attente au rafraîchissement. `crates/whisper-core/src/application/runtime/poll.rs:1-45` | 45 | 0 |
| crate::application::runtime::queue | Traite file, historique, doublons, admissions et résolutions explicites. `crates/whisper-core/src/application/runtime/queue.rs:1-476` | 476 | 0 |
| crate::application::runtime::tests | Tests ou supports : définit `close_blocks_direct_and_indirect_admissions_without_submitting_effects` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:27-116`, `empty_snapshot_recovery_inspection_closes_dialog_and_reports_result` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:119-133`. | 0 | 313 |
| crate::application::runtime::tests | Tests ou supports : définit `repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation` `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:2-24`, `every_job_lifecycle_command_carries_identity` `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:27-87`. | 0 | 765 |
| crate::application::runtime::tests | Tests ou supports : définit `parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event` `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:2-78`, `parakeet_automatic_final_import_stays_unverified_and_separate_from_confirmed_text` `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:81-383`. | 0 | 954 |
| crate::application::runtime::tests | Tests ou supports : définit `unified_page_responses_and_errors_follow_latest_selection_request` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:2-216`, `unified_session_card_cache_stays_bounded_at_eight` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:219-243`. | 0 | 1146 |
| crate::application::runtime::tests | Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-core/src/application/runtime/tests.rs:1-4`. | 0 | 4 |
| crate::application::runtime::unified | Projette sessions unifiées, pages document/audio et commandes de capture. `crates/whisper-core/src/application/runtime/unified.rs:1-1128` | 1128 | 0 |
| crate::application | Réexporte contrats et ImportApplication depuis ses modules privés. `crates/whisper-core/src/application.rs:1-9` | 9 | 0 |
| crate::audio | Reconnaît les fins audio courtes et silencieuses par calcul d’énergie. `crates/whisper-core/src/audio.rs:1-45` | 26 | 19 |
| crate::compute_policy | Décide le backend initial et les suites permises après son échec. `crates/whisper-core/src/compute_policy.rs:1-67` | 67 | 0 |
| crate::domain | Définit identités, générations, plages audio et transitions JobState. `crates/whisper-core/src/domain.rs:1-170` | 122 | 48 |
| crate::engine_capabilities | Projette et valide les combinaisons moteurs, modèles et capacités. `crates/whisper-core/src/engine_capabilities.rs:1-527` | 527 | 0 |
| crate::gpu_inventory | Qualifie et normalise observations et identités GPU. `crates/whisper-core/src/gpu_inventory.rs:1-341` | 341 | 0 |
| crate::gpu_inventory_app | Suit une collecte GPU via un port et expose son état et sa sélection. `crates/whisper-core/src/gpu_inventory_app.rs:1-248` | 248 | 0 |
| crate::gpu_selection | Résout les préférences GPU et vérifie les attestations du candidat. `crates/whisper-core/src/gpu_selection.rs:1-204` | 204 | 0 |
| crate::ipc | Définit enveloppes, commandes et événements du protocole Turbo. `crates/whisper-core/src/ipc.rs:1-343` | 326 | 17 |
| crate | Déclare les modules et interfaces de bibliothèque. `crates/whisper-core/src/lib.rs:1-45` | 45 | 0 |
| crate::microphone | Définit le port microphone et conserve choix préféré/effectif et notifications. `crates/whisper-core/src/microphone.rs:1-247` | 161 | 86 |
| crate::microphone_level | Définit gain, identités/révisions, mesures de niveau et de spectre. `crates/whisper-core/src/microphone_level.rs:1-628` | 548 | 80 |
| crate::model_catalog | Définit identités, disponibilités et observations de modèles. `crates/whisper-core/src/model_catalog.rs:1-50` | 50 | 0 |
| crate::model_registry | Déclare les modèles et construit leurs clés de configuration. `crates/whisper-core/src/model_registry.rs:1-345` | 345 | 0 |
| crate::model_selection_app | Valide les demandes import/live contre catalogue et capacités. `crates/whisper-core/src/model_selection_app.rs:1-313` | 313 | 0 |
| crate::parakeet_ipc | Définit commandes, événements, identité et preuves GPU du protocole Parakeet. `crates/whisper-core/src/parakeet_ipc.rs:1-130` | 130 | 0 |
| crate::parakeet_scheduler | Ordonne les appels preview/final et suit admission, exécution et recyclage. `crates/whisper-core/src/parakeet_scheduler.rs:1-672` | 424 | 248 |
| crate::parakeet_session | Définit stratégies et transitions des frontières/fenêtres de session. `crates/whisper-core/src/parakeet_session.rs:1-433` | 309 | 124 |
| crate::parakeet_windows | Planifie les fenêtres et convertit les temps des mots en échantillons absolus. `crates/whisper-core/src/parakeet_windows.rs:1-333` | 273 | 60 |
| crate::path_settings | Restaure et mémorise chemins et préférences au travers de ports. `crates/whisper-core/src/path_settings.rs:1-851` | 616 | 235 |
| crate::ports | Définit les ports et données échangées par effets et événements. `crates/whisper-core/src/ports.rs:1-873` | 873 | 0 |
| integration::application_routing | Tests ou supports : définit `legacy_start_live_is_rejected_without_effects` `crates/whisper-core/tests/application_routing.rs:30-46`, `history_event_is_applied_without_an_active_job` `crates/whisper-core/tests/application_routing.rs:49-70`. | 0 | 70 |
| integration::compute_policy | Tests ou supports : définit `forced_cpu_never_selects_or_initializes_gpu` `crates/whisper-core/tests/compute_policy.rs:6-15`, `auto_uses_gpu_when_available_and_allows_only_gpu_to_cpu_fallback` `crates/whisper-core/tests/compute_policy.rs:18-47`. | 0 | 63 |
| integration::engine_capabilities | Tests ou supports : définit `auto_and_manual_language_are_independent_and_projection_matches_admission` `crates/whisper-core/tests/engine_capabilities.rs:115-224`, `no_admissible_language_mode_refuses_launch_and_unknown_is_not_unsupported` `crates/whisper-core/tests/engine_capabilities.rs:227-291`. | 0 | 718 |
| integration::gpu_inventory | Tests ou supports : définit `inventory_keeps_homonymous_devices_and_excludes_confirmed_absence` `crates/whisper-core/tests/gpu_inventory.rs:36-49`, `current_device_with_missing_properties_is_kept_as_unknown` `crates/whisper-core/tests/gpu_inventory.rs:52-66`. | 0 | 400 |
| integration::gpu_selection | Tests ou supports : définit `default_uses_windows_rank_even_when_motor_order_and_names_are_reversed` `crates/whisper-core/tests/gpu_selection.rs:45-76`, `preferred_compatible_device_wins_without_rank_and_missing_rank_never_becomes_default` `crates/whisper-core/tests/gpu_selection.rs:79-93`. | 0 | 297 |
| integration::import_contract | Tests ou supports : définit `application_keeps_bounded_strategy_in_prepare_and_start_effects` `crates/whisper-core/tests/import_contract.rs:49-72`, `application_publishes_only_after_end_and_synced_segment_receipt` `crates/whisper-core/tests/import_contract.rs:75-168`. | 0 | 902 |
| integration::live_contract | Tests ou supports : définit `starting_live_emits_one_explicit_effect_and_initializes_separate_counters` `crates/whisper-core/tests/live_contract.rs:47-76`, `forced_gpu_recovery_requires_an_explicit_cpu_finish_command` `crates/whisper-core/tests/live_contract.rs:79-109`. | 0 | 233 |
| integration::microphone_level | Tests ou supports : définit `spectrum_context_binds_source_and_rejects_stale_foreign_and_future_updates` `crates/whisper-core/tests/microphone_level.rs:13-81`, `retry_barrier_rejects_same_owner_spectrum_acquired_before_cpu_retry` `crates/whisper-core/tests/microphone_level.rs:84-140`. | 0 | 238 |
| integration::model_catalog | Tests ou supports : définit `catalog_is_closed_to_the_two_planned_model_identities` `crates/whisper-core/tests/model_catalog.rs:61-68`, `configuration_identity_includes_every_qualification_dimension` `crates/whisper-core/tests/model_catalog.rs:71-115`. | 0 | 242 |
| integration::model_preferences | Tests ou supports : définit `turbo_is_default_and_user_model_and_auto_preferences_are_saved_with_revisions` `crates/whisper-core/tests/model_preferences.rs:51-71`, `failed_or_stale_preference_ack_is_reported_without_reverting_the_session_choice` `crates/whisper-core/tests/model_preferences.rs:74-99`. | 0 | 120 |
| integration::model_selection_app | Tests ou supports : définit `turbo_existing_matrix_admits_all_1206_inputs_without_native_inference` `crates/whisper-core/tests/model_selection_app.rs:20-61`, `turbo_existing_matrix_admits_and_submits_all_1206_guard_requests` `crates/whisper-core/tests/model_selection_app.rs:64-144`. | 0 | 430 |
| integration::parakeet_import_contract | Tests ou supports : définit `only_qualified_cpu_import_with_automatic_language_is_admitted` `crates/whisper-core/tests/parakeet_import_contract.rs:71-119`, `parakeet_import_admits_long_product_input_but_rejects_wrong_hash` `crates/whisper-core/tests/parakeet_import_contract.rs:122-136`. | 0 | 278 |
| integration::parakeet_ipc | Tests ou supports : définit `parakeet_protocol_is_versioned_and_caps_are_explicit` `crates/whisper-core/tests/parakeet_ipc.rs:8-13`, `transcribe_command_carries_complete_build_runtime_profile_and_attempt_identity` `crates/whisper-core/tests/parakeet_ipc.rs:16-49`. | 0 | 113 |
| integration::parakeet_windows | Tests ou supports : définit `silence_threshold_and_minimum_duration_are_inclusive` `crates/whisper-core/tests/parakeet_windows.rs:18-31`, `chooses_earliest_of_equal_distance_silences_near_ninety_seconds` `crates/whisper-core/tests/parakeet_windows.rs:34-45`. | 0 | 194 |
| integration::scheduler_contract | Tests ou supports : définit `admission_and_restoration_never_start_a_worker_without_user_choice` `crates/whisper-core/tests/scheduler_contract.rs:43-77`, `enqueue_is_acknowledged_only_after_the_durable_queue_event` `crates/whisper-core/tests/scheduler_contract.rs:80-114`. | 0 | 154 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| automatic_final | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/automatic_final.rs | `crates/whisper-core/src/application/runtime/mod.rs:22` |
| closing | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/closing.rs | `crates/whisper-core/src/application/runtime/mod.rs:23` |
| commands | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/commands.rs | `crates/whisper-core/src/application/runtime/mod.rs:24` |
| events | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/events.rs | `crates/whisper-core/src/application/runtime/mod.rs:25` |
| import | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/import.rs | `crates/whisper-core/src/application/runtime/mod.rs:26` |
| live | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/live.rs | `crates/whisper-core/src/application/runtime/mod.rs:27` |
| microphone | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/microphone.rs | `crates/whisper-core/src/application/runtime/mod.rs:28` |
| parakeet_views | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/parakeet_views.rs | `crates/whisper-core/src/application/runtime/mod.rs:29` |
| poll | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/poll.rs | `crates/whisper-core/src/application/runtime/mod.rs:30` |
| queue | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/queue.rs | `crates/whisper-core/src/application/runtime/mod.rs:31` |
| unified | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/unified.rs | `crates/whisper-core/src/application/runtime/mod.rs:32` |
| tests | externe | tests | crates/whisper-core/src/application/runtime/tests.rs | `crates/whisper-core/src/application/runtime/mod.rs:149-150` |
| contracts | externe | hors tests / cfg | crates/whisper-core/src/application/contracts.rs | `crates/whisper-core/src/application.rs:1` |
| event_metadata | externe | hors tests / cfg | crates/whisper-core/src/application/event_metadata.rs | `crates/whisper-core/src/application.rs:2` |
| runtime | externe | hors tests / cfg | crates/whisper-core/src/application/runtime/mod.rs | `crates/whisper-core/src/application.rs:3` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/audio.rs:27-45` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/domain.rs:123-170` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/ipc.rs:327-343` |
| application | externe | hors tests / cfg | crates/whisper-core/src/application.rs | `crates/whisper-core/src/lib.rs:3` |
| audio | externe | hors tests / cfg | crates/whisper-core/src/audio.rs | `crates/whisper-core/src/lib.rs:4` |
| compute_policy | externe | hors tests / cfg | crates/whisper-core/src/compute_policy.rs | `crates/whisper-core/src/lib.rs:5` |
| domain | externe | hors tests / cfg | crates/whisper-core/src/domain.rs | `crates/whisper-core/src/lib.rs:6` |
| engine_capabilities | externe | hors tests / cfg | crates/whisper-core/src/engine_capabilities.rs | `crates/whisper-core/src/lib.rs:7` |
| gpu_inventory | externe | hors tests / cfg | crates/whisper-core/src/gpu_inventory.rs | `crates/whisper-core/src/lib.rs:8` |
| gpu_inventory_app | externe | hors tests / cfg | crates/whisper-core/src/gpu_inventory_app.rs | `crates/whisper-core/src/lib.rs:9` |
| gpu_selection | externe | hors tests / cfg | crates/whisper-core/src/gpu_selection.rs | `crates/whisper-core/src/lib.rs:10` |
| ipc | externe | hors tests / cfg | crates/whisper-core/src/ipc.rs | `crates/whisper-core/src/lib.rs:11` |
| microphone | externe | hors tests / cfg | crates/whisper-core/src/microphone.rs | `crates/whisper-core/src/lib.rs:12` |
| microphone_level | externe | hors tests / cfg | crates/whisper-core/src/microphone_level.rs | `crates/whisper-core/src/lib.rs:13` |
| model_catalog | externe | hors tests / cfg | crates/whisper-core/src/model_catalog.rs | `crates/whisper-core/src/lib.rs:14` |
| model_registry | externe | hors tests / cfg | crates/whisper-core/src/model_registry.rs | `crates/whisper-core/src/lib.rs:15` |
| model_selection_app | externe | hors tests / cfg | crates/whisper-core/src/model_selection_app.rs | `crates/whisper-core/src/lib.rs:16` |
| parakeet_ipc | externe | hors tests / cfg | crates/whisper-core/src/parakeet_ipc.rs | `crates/whisper-core/src/lib.rs:17` |
| parakeet_scheduler | externe | hors tests / cfg | crates/whisper-core/src/parakeet_scheduler.rs | `crates/whisper-core/src/lib.rs:18` |
| parakeet_session | externe | hors tests / cfg | crates/whisper-core/src/parakeet_session.rs | `crates/whisper-core/src/lib.rs:19` |
| parakeet_windows | externe | hors tests / cfg | crates/whisper-core/src/parakeet_windows.rs | `crates/whisper-core/src/lib.rs:20` |
| path_settings | externe | hors tests / cfg | crates/whisper-core/src/path_settings.rs | `crates/whisper-core/src/lib.rs:21` |
| ports | externe | hors tests / cfg | crates/whisper-core/src/ports.rs | `crates/whisper-core/src/lib.rs:22` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/microphone.rs:162-247` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/microphone_level.rs:549-628` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/parakeet_scheduler.rs:425-672` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/parakeet_session.rs:310-433` |
| tests | inline | tests | plage locale | `crates/whisper-core/src/parakeet_windows.rs:274-333` |
| microphone_tests | inline | tests | plage locale | `crates/whisper-core/src/path_settings.rs:617-851` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate::application::contracts — `crates/whisper-core/src/application/contracts.rs`

Définit AppCommand, AppView, Application, AppFacade et les phases capture/moteur. `crates/whisper-core/src/application/contracts.rs:1-318`

Mesure : 318 production / 0 tests / 318 physiques. `crates/whisper-core/src/application/contracts.rs:1-318`

Types : `CommandIdentity` (struct, hors tests) `crates/whisper-core/src/application/contracts.rs:7-10`; `AppCommand` (enum, hors tests) `crates/whisper-core/src/application/contracts.rs:13-118`; `AppView` (struct, hors tests) `crates/whisper-core/src/application/contracts.rs:198-251`; `DirectCapturePhase` (enum, hors tests) `crates/whisper-core/src/application/contracts.rs:254-259`; `DirectEnginePhase` (enum, hors tests) `crates/whisper-core/src/application/contracts.rs:262-267`; `ApplicationError` (enum, hors tests) `crates/whisper-core/src/application/contracts.rs:270-274`; `Application` (trait, hors tests) `crates/whisper-core/src/application/contracts.rs:277-290`; `AppFacade` (struct, hors tests) `crates/whisper-core/src/application/contracts.rs:292-294`.

Fonctions avec corps : `identity` `crates/whisper-core/src/application/contracts.rs:121-190`; `targets` `crates/whisper-core/src/application/contracts.rs:192-194`; `set_gpu_candidate` `crates/whisper-core/src/application/contracts.rs:281-289`; `new` `crates/whisper-core/src/application/contracts.rs:297-299`; `dispatch` `crates/whisper-core/src/application/contracts.rs:301-303`; `view` `crates/whisper-core/src/application/contracts.rs:305-307`; `set_gpu_candidate` `crates/whisper-core/src/application/contracts.rs:309-317`.

Déclarations `pub` (visibilité sous conditions) : `pub struct CommandIdentity {` `crates/whisper-core/src/application/contracts.rs:7`; `pub enum AppCommand {` `crates/whisper-core/src/application/contracts.rs:13`; `pub fn identity(&self) -> Option<CommandIdentity> {` `crates/whisper-core/src/application/contracts.rs:121`; `pub fn targets(&self, job_id: JobId, generation: Generation) -> bool {` `crates/whisper-core/src/application/contracts.rs:192`; `pub struct AppView {` `crates/whisper-core/src/application/contracts.rs:198`; `pub enum DirectCapturePhase {` `crates/whisper-core/src/application/contracts.rs:254`; `pub enum DirectEnginePhase {` `crates/whisper-core/src/application/contracts.rs:262`; `pub enum ApplicationError {` `crates/whisper-core/src/application/contracts.rs:270`; `pub trait Application {` `crates/whisper-core/src/application/contracts.rs:277`; `pub struct AppFacade<A> {` `crates/whisper-core/src/application/contracts.rs:292`; `pub fn new(application: A) -> Self {` `crates/whisper-core/src/application/contracts.rs:297`; `pub fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {` `crates/whisper-core/src/application/contracts.rs:301`; `pub fn view(&self) -> AppView {` `crates/whisper-core/src/application/contracts.rs:305`; `pub fn set_gpu_candidate(` `crates/whisper-core/src/application/contracts.rs:309`.

Dépend de, via `use crate::` : hors tests : `crate::domain::Generation` → `crate::domain` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::domain::JobConfig` → `crate::domain` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::domain::JobId` → `crate::domain` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::domain::JobState` → `crate::domain` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::ports::ArchiveHistoryItem` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::DuplicateImportChoice` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::DuplicateImportPrompt` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::ImportQueueEntry` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::ImportRequest` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::LiveRequest` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::ReplacementCleanupInfo` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports::SourceSnapshotRecoveryCandidate` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::ports` → `crate::ports` `crates/whisper-core/src/application/contracts.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::event_metadata — `crates/whisper-core/src/application/event_metadata.rs`

Associe les variantes ImportEvent à un nom et une identité job/génération. `crates/whisper-core/src/application/event_metadata.rs:1-170`

Mesure : 170 production / 0 tests / 170 physiques. `crates/whisper-core/src/application/event_metadata.rs:1-170`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/event_metadata.rs:1-170`.

Fonctions avec corps : `kind_name` `crates/whisper-core/src/application/event_metadata.rs:5-56`; `identity` `crates/whisper-core/src/application/event_metadata.rs:60-169`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/event_metadata.rs:1-170`.

Dépend de, via `use crate::` : hors tests : `crate::domain::Generation` → `crate::domain` `crates/whisper-core/src/application/event_metadata.rs:1`<br>hors tests : `crate::domain::JobId` → `crate::domain` `crates/whisper-core/src/application/event_metadata.rs:1`<br>hors tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/application/event_metadata.rs:1`<br>hors tests : `crate::ports::ImportEvent` → `crate::ports` `crates/whisper-core/src/application/event_metadata.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::automatic_final — `crates/whisper-core/src/application/runtime/automatic_final.rs`

Suit l’import final automatique et sa persistance jusqu’à publication. `crates/whisper-core/src/application/runtime/automatic_final.rs:1-381`

Mesure : 381 production / 0 tests / 381 physiques. `crates/whisper-core/src/application/runtime/automatic_final.rs:1-381`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/automatic_final.rs:1-381`.

Fonctions avec corps : `track_automatic_final_event` `crates/whisper-core/src/application/runtime/automatic_final.rs:4-34`; `set_capture_automatic_final_status` `crates/whisper-core/src/application/runtime/automatic_final.rs:36-53`; `mark_automatic_final_failed` `crates/whisper-core/src/application/runtime/automatic_final.rs:55-71`; `start_next_automatic_final` `crates/whisper-core/src/application/runtime/automatic_final.rs:73-102`; `retry_automatic_final_segment` `crates/whisper-core/src/application/runtime/automatic_final.rs:104-145`; `handle_automatic_final_event` `crates/whisper-core/src/application/runtime/automatic_final.rs:147-171`; `handle_automatic_final_preparation` `crates/whisper-core/src/application/runtime/automatic_final.rs:173-226`; `handle_automatic_final_persistence` `crates/whisper-core/src/application/runtime/automatic_final.rs:228-311`; `handle_automatic_final_terminal` `crates/whisper-core/src/application/runtime/automatic_final.rs:313-348`; `maybe_publish_automatic_final` `crates/whisper-core/src/application/runtime/automatic_final.rs:350-380`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/automatic_final.rs:1-381`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/automatic_final.rs:1-381`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::closing — `crates/whisper-core/src/application/runtime/closing.rs`

Pilote la demande de fermeture, les retraits de file et les arrêts actifs. `crates/whisper-core/src/application/runtime/closing.rs:1-184`

Mesure : 184 production / 0 tests / 184 physiques. `crates/whisper-core/src/application/runtime/closing.rs:1-184`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/closing.rs:1-184`.

Fonctions avec corps : `update_close_ready` `crates/whisper-core/src/application/runtime/closing.rs:4-19`; `drive_close_queue_removals` `crates/whisper-core/src/application/runtime/closing.rs:21-50`; `request_close` `crates/whisper-core/src/application/runtime/closing.rs:52-103`; `stop_or_quit` `crates/whisper-core/src/application/runtime/closing.rs:105-183`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/closing.rs:1-184`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/closing.rs:1-184`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::commands — `crates/whisper-core/src/application/runtime/commands.rs`

Vérifie l’admission des modèles et route les commandes Application. `crates/whisper-core/src/application/runtime/commands.rs:1-145`

Mesure : 145 production / 0 tests / 145 physiques. `crates/whisper-core/src/application/runtime/commands.rs:1-145`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/commands.rs:1-145`.

Fonctions avec corps : `refuse_model_command` `crates/whisper-core/src/application/runtime/commands.rs:4-7`; `dispatch_admitted` `crates/whisper-core/src/application/runtime/commands.rs:9-69`; `set_gpu_candidate` `crates/whisper-core/src/application/runtime/commands.rs:73-90`; `dispatch` `crates/whisper-core/src/application/runtime/commands.rs:92-140`; `view` `crates/whisper-core/src/application/runtime/commands.rs:142-144`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/commands.rs:1-145`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/commands.rs:1-145`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::events — `crates/whisper-core/src/application/runtime/events.rs`

Route les événements import, micro, capture et finalisation. `crates/whisper-core/src/application/runtime/events.rs:1-130`

Mesure : 130 production / 0 tests / 130 physiques. `crates/whisper-core/src/application/runtime/events.rs:1-130`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/events.rs:1-130`.

Fonctions avec corps : `handle_event` `crates/whisper-core/src/application/runtime/events.rs:4-59`; `route_active_event` `crates/whisper-core/src/application/runtime/events.rs:61-129`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/events.rs:1-130`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/events.rs:1-130`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::import — `crates/whisper-core/src/application/runtime/import.rs`

Gère préparation, segments, acquittements, publication et reprise import. `crates/whisper-core/src/application/runtime/import.rs:1-515`

Mesure : 515 production / 0 tests / 515 physiques. `crates/whisper-core/src/application/runtime/import.rs:1-515`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/import.rs:1-515`.

Fonctions avec corps : `handle_import_event` `crates/whisper-core/src/application/runtime/import.rs:4-24`; `handle_import_preparation` `crates/whisper-core/src/application/runtime/import.rs:26-70`; `handle_import_progress` `crates/whisper-core/src/application/runtime/import.rs:72-140`; `handle_import_persistence` `crates/whisper-core/src/application/runtime/import.rs:142-198`; `handle_import_terminal` `crates/whisper-core/src/application/runtime/import.rs:200-304`; `maybe_publish` `crates/whisper-core/src/application/runtime/import.rs:306-325`; `accept_segment` `crates/whisper-core/src/application/runtime/import.rs:327-347`; `retry_pending_segment` `crates/whisper-core/src/application/runtime/import.rs:351-375`; `handle_start_import_command` `crates/whisper-core/src/application/runtime/import.rs:376-421`; `handle_resume_import_command` `crates/whisper-core/src/application/runtime/import.rs:423-514`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/import.rs:1-515`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/import.rs:1-515`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::live — `crates/whisper-core/src/application/runtime/live.rs`

Traite commandes/événements live, compteurs audio et finalisation CPU. `crates/whisper-core/src/application/runtime/live.rs:1-720`

Mesure : 720 production / 0 tests / 720 physiques. `crates/whisper-core/src/application/runtime/live.rs:1-720`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/live.rs:1-720`.

Fonctions avec corps : `handle_live_event` `crates/whisper-core/src/application/runtime/live.rs:4-46`; `handle_direct_event` `crates/whisper-core/src/application/runtime/live.rs:48-131`; `handle_parakeet_start_event` `crates/whisper-core/src/application/runtime/live.rs:133-175`; `handle_parakeet_progress_event` `crates/whisper-core/src/application/runtime/live.rs:177-245`; `handle_parakeet_audio_event` `crates/whisper-core/src/application/runtime/live.rs:247-292`; `handle_parakeet_stop_event` `crates/whisper-core/src/application/runtime/live.rs:294-362`; `handle_live_counter_event` `crates/whisper-core/src/application/runtime/live.rs:364-422`; `handle_live_reset_event` `crates/whisper-core/src/application/runtime/live.rs:424-465`; `handle_live_retry_event` `crates/whisper-core/src/application/runtime/live.rs:467-507`; `enqueue_capture_final` `crates/whisper-core/src/application/runtime/live.rs:509-568`; `start_live_command` `crates/whisper-core/src/application/runtime/live.rs:570-646`; `initialize_live_view` `crates/whisper-core/src/application/runtime/live.rs:648-693`; `finish_live_cpu_command` `crates/whisper-core/src/application/runtime/live.rs:695-719`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/live.rs:1-720`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/live.rs:1-720`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::microphone — `crates/whisper-core/src/application/runtime/microphone.rs`

Gère commandes/événements de test microphone et de gain. `crates/whisper-core/src/application/runtime/microphone.rs:1-370`

Mesure : 370 production / 0 tests / 370 physiques. `crates/whisper-core/src/application/runtime/microphone.rs:1-370`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/microphone.rs:1-370`.

Fonctions avec corps : `invalidate_microphone_acquisition` `crates/whisper-core/src/application/runtime/microphone.rs:4-27`; `handle_microphone_event` `crates/whisper-core/src/application/runtime/microphone.rs:29-41`; `handle_microphone_gain_event` `crates/whisper-core/src/application/runtime/microphone.rs:43-76`; `handle_microphone_test_event` `crates/whisper-core/src/application/runtime/microphone.rs:78-166`; `handle_unified_microphone_event` `crates/whisper-core/src/application/runtime/microphone.rs:168-240`; `handle_microphone_test_command` `crates/whisper-core/src/application/runtime/microphone.rs:242-304`; `handle_microphone_gain_command` `crates/whisper-core/src/application/runtime/microphone.rs:306-369`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/microphone.rs:1-370`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/microphone.rs:1-370`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime — `crates/whisper-core/src/application/runtime/mod.rs`

Porte l’état ImportApplication et construit files, compteurs et vues. `crates/whisper-core/src/application/runtime/mod.rs:1-150`

Mesure : 140 production / 10 tests / 150 physiques. `crates/whisper-core/src/application/runtime/mod.rs:1-150`

Types : `ImportApplication` (struct, hors tests) `crates/whisper-core/src/application/runtime/mod.rs:35-66`; `UnifiedCaptureAdmission` (struct, hors tests) `crates/whisper-core/src/application/runtime/mod.rs:68-81`; `AutomaticFinalJobState` (struct, hors tests) `crates/whisper-core/src/application/runtime/mod.rs:83-92`.

Fonctions avec corps : `new` `crates/whisper-core/src/application/runtime/mod.rs:99-132`; `with_model_guard` `crates/whisper-core/src/application/runtime/mod.rs:135-141`; `fail` `crates/whisper-core/src/application/runtime/mod.rs:143-146`.

Déclarations `pub` (visibilité sous conditions) : `pub struct ImportApplication<P> {` `crates/whisper-core/src/application/runtime/mod.rs:35`; `pub fn new(io: P) -> Self {` `crates/whisper-core/src/application/runtime/mod.rs:99`; `pub fn with_model_guard(` `crates/whisper-core/src/application/runtime/mod.rs:135`.

Dépend de, via `use crate::` : hors tests : `crate::domain::Generation` → `crate::domain` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::domain::JobId` → `crate::domain` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::domain::JobState` → `crate::domain` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::ports::DiagnosticNotice` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports::ImportEffect` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports::ImportEvent` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports::ImportIoPort` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports::ImportQueueStatus` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports::ImportRequest` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports::PortError` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::ports` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>tests : `crate::domain::JobConfig` → `crate::domain` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::ports::DuplicateImportChoice` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::ports::DuplicateImportPrompt` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::ports::ImportQueueEntry` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::ports::LiveRequest` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::ports::SourceSnapshotRecoveryCandidate` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::ports` → `crate::ports` `crates/whisper-core/src/application/runtime/mod.rs:7`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::parakeet_views — `crates/whisper-core/src/application/runtime/parakeet_views.rs`

Actualise les cartes de captures Parakeet et leurs textes/états. `crates/whisper-core/src/application/runtime/parakeet_views.rs:1-572`

Mesure : 572 production / 0 tests / 572 physiques. `crates/whisper-core/src/application/runtime/parakeet_views.rs:1-572`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/parakeet_views.rs:1-572`.

Fonctions avec corps : `track_parakeet_capture_event` `crates/whisper-core/src/application/runtime/parakeet_views.rs:5-20`; `ensure_parakeet_capture` `crates/whisper-core/src/application/runtime/parakeet_views.rs:21-51`; `track_capture_start` `crates/whisper-core/src/application/runtime/parakeet_views.rs:53-106`; `track_capture_text` `crates/whisper-core/src/application/runtime/parakeet_views.rs:108-186`; `track_capture_ranges` `crates/whisper-core/src/application/runtime/parakeet_views.rs:188-232`; `track_capture_final` `crates/whisper-core/src/application/runtime/parakeet_views.rs:234-332`; `apply_final_segments` `crates/whisper-core/src/application/runtime/parakeet_views.rs:334-374`; `track_capture_failure` `crates/whisper-core/src/application/runtime/parakeet_views.rs:376-445`; `track_capture_stopped` `crates/whisper-core/src/application/runtime/parakeet_views.rs:447-485`; `find_parakeet_capture_mut` `crates/whisper-core/src/application/runtime/parakeet_views.rs:487-498`; `trim_parakeet_capture_views` `crates/whisper-core/src/application/runtime/parakeet_views.rs:500-515`; `append_parakeet_confirmed` `crates/whisper-core/src/application/runtime/parakeet_views.rs:517-571`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/parakeet_views.rs:1-572`.

Dépend de, via `use crate::` : hors tests : `crate::ports::ParakeetCaptureIdentity` → `crate::ports` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::ports::ParakeetCaptureView` → `crate::ports` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::ports::ParakeetCaptureViewState` → `crate::ports` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::ports` → `crate::ports` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::poll — `crates/whisper-core/src/application/runtime/poll.rs`

Draine les événements du port et les segments en attente au rafraîchissement. `crates/whisper-core/src/application/runtime/poll.rs:1-45`

Mesure : 45 production / 0 tests / 45 physiques. `crates/whisper-core/src/application/runtime/poll.rs:1-45`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/poll.rs:1-45`.

Fonctions avec corps : `poll_once` `crates/whisper-core/src/application/runtime/poll.rs:4-44`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/poll.rs:1-45`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/poll.rs:1-45`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::queue — `crates/whisper-core/src/application/runtime/queue.rs`

Traite file, historique, doublons, admissions et résolutions explicites. `crates/whisper-core/src/application/runtime/queue.rs:1-476`

Mesure : 476 production / 0 tests / 476 physiques. `crates/whisper-core/src/application/runtime/queue.rs:1-476`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/queue.rs:1-476`.

Fonctions avec corps : `handle_queue_event` `crates/whisper-core/src/application/runtime/queue.rs:4-25`; `handle_history_event` `crates/whisper-core/src/application/runtime/queue.rs:27-111`; `handle_queue_admission_event` `crates/whisper-core/src/application/runtime/queue.rs:113-184`; `handle_duplicate_event` `crates/whisper-core/src/application/runtime/queue.rs:186-204`; `handle_queue_failure_event` `crates/whisper-core/src/application/runtime/queue.rs:206-241`; `handle_queue_command` `crates/whisper-core/src/application/runtime/queue.rs:243-265`; `handle_history_command` `crates/whisper-core/src/application/runtime/queue.rs:267-311`; `handle_queue_removal_command` `crates/whisper-core/src/application/runtime/queue.rs:313-357`; `handle_enqueue_command` `crates/whisper-core/src/application/runtime/queue.rs:359-417`; `handle_queue_resolution_command` `crates/whisper-core/src/application/runtime/queue.rs:419-475`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/queue.rs:1-476`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/queue.rs:1-476`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::tests — `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs`

Tests ou supports : définit `close_blocks_direct_and_indirect_admissions_without_submitting_effects` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:27-116`, `empty_snapshot_recovery_inspection_closes_dialog_and_reports_result` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:119-133`.

Mesure : 0 production / 313 tests / 313 physiques. `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:1-313`

Types : `ScanTestIo` (struct, tests) `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:6-10`.

Fonctions avec corps : `submit` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:13-19`; `poll` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:21-23`; `close_blocks_direct_and_indirect_admissions_without_submitting_effects` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:27-116`; `empty_snapshot_recovery_inspection_closes_dialog_and_reports_result` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:119-133`; `snapshot_recovery_completion_closes_dialog_and_keeps_error_and_protected_reason_visible` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:136-160`; `successful_snapshot_recovery_completion_closes_dialog_and_reports_recovered_copies` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:163-181`; `close_removes_durable_queued_work_before_reporting_ready` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:184-221`; `close_keeps_failed_queue_removal_recoverable_and_retries_it` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:224-276`; `request_close_waits_for_an_already_committing_import_result` [tests] `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:279-313`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:1-313`.

Dépend de, via `use crate::` : tests : `crate::domain::ComputeChoice` → `crate::domain` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:2`<br>tests : `crate::domain::LanguageChoice` → `crate::domain` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:2`<br>tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::tests — `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs`

Tests ou supports : définit `repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation` `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:2-24`, `every_job_lifecycle_command_carries_identity` `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:27-87`.

Mesure : 0 production / 765 tests / 765 physiques. `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:1-765`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:1-765`.

Fonctions avec corps : `repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:2-24`; `every_job_lifecycle_command_carries_identity` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:27-87`; `live_stop_keeps_generation_until_segments_are_persisted_and_published` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:90-160`; `final_stop_counters_allow_last_window_and_match_published_view` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:163-225`; `live_state_advances_through_capture_drain_and_finalization` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:228-280`; `immediate_live_stop_does_not_reenter_capturing_on_late_ready` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:283-323`; `live_auto_stop_acts_only_when_a_positive_speech_limit_is_configured` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:326-380`; `live_provisional_text_is_exposed_and_persisted_by_the_application` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:383-435`; `import_can_be_enqueued_during_live_without_replacing_the_live_job` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:438-481`; `queued_import_starts_only_after_live_publication_and_explicit_processing` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:484-557`; `stopped_import_ignores_late_segments_and_remains_cancelled` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:560-602`; `direct_capture_and_engine_progress_independently_through_pre_ready_stop` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:605-674`; `live_capture_failure_remains_recoverable_with_its_diagnostic` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:677-708`; `failed_live_stop_retains_recoverable_diagnostic_and_confirmed_fragment` [tests] `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:711-765`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:1-765`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/tests/live_lifecycle.rs:1-765`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::tests — `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs`

Tests ou supports : définit `parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event` `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:2-78`, `parakeet_automatic_final_import_stays_unverified_and_separate_from_confirmed_text` `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:81-383`.

Mesure : 0 production / 954 tests / 954 physiques. `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:1-954`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:1-954`.

Fonctions avec corps : `parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:2-78`; `parakeet_automatic_final_import_stays_unverified_and_separate_from_confirmed_text` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:81-383`; `late_automatic_import_failure_updates_only_its_capture_row` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:386-550`; `direct_start_failure_before_live_started_keeps_a_capture_row_without_wav` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:553-657`; `turbo_start_failure_before_live_started_does_not_create_parakeet_capture` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:660-712`; `parakeet_late_confirmation_keeps_identity_and_reorders_display_chronologically` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:715-748`; `unified_capture_state` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:750-768`; `late_unified_gain_rejection_does_not_rollback_newer_capture_view` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:771-823`; `prepared_unified_capture` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:825-841`; `unified_capture_ready_records_the_effective_native_backend` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:844-876`; `unified_capture_prepared_progress_end_publishes_only_after_total_is_known` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:879-910`; `unified_capture_rejects_progress_with_a_total_different_from_prepared_source` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:913-934`; `unified_capture_rejects_end_until_prepared_total_arrives` [tests] `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:937-954`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:1-954`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs:1-954`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::tests — `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs`

Tests ou supports : définit `unified_page_responses_and_errors_follow_latest_selection_request` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:2-216`, `unified_session_card_cache_stays_bounded_at_eight` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:219-243`.

Mesure : 0 production / 1146 tests / 1146 physiques. `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:1-1146`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:1-1146`.

Fonctions avec corps : `unified_page_responses_and_errors_follow_latest_selection_request` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:2-216`; `unified_session_card_cache_stays_bounded_at_eight` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:219-243`; `microphone_gain_command_is_scoped_to_the_active_direct_capture` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:246-366`; `microphone_test_gain_requested_during_open_survives_started_event` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:369-448`; `microphone_test_owns_one_capture_then_releases_before_live_restart` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:451-663`; `microphone_test_failure_invalidates_its_meter_and_source` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:666-694`; `destination_scan_stays_pending_until_history_and_queue_are_consumed` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:697-716`; `queue_scan_failure_clears_pending_state_and_is_visible` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:719-740`; `cancelling_duplicate_import_releases_only_the_new_source_snapshot` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:743-788`; `stale_duplicate_dialog_response_cannot_submit_a_replacement` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:791-837`; `model_guard_blocks_each_import_and_live_effect_route_before_submission` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:840-911`; `turbo_existing_matrix_dispatches_all_1206_requests_to_effect_submission` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:914-997`; `stale_audio_pages_reset_revision_and_preserve_real_errors_and_request_identity` [tests] `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:1000-1146`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:1-1146`.

Dépend de, via `use crate::` : tests : `crate::model_catalog::ModelAvailability` → `crate::model_catalog` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_catalog::ModelId` → `crate::model_catalog` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_catalog` → `crate::model_catalog` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_registry::TURBO_LANGUAGE_INPUTS` → `crate::model_registry` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_registry::engine_snapshot` → `crate::model_registry` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_registry::model` → `crate::model_registry` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_registry` → `crate::model_registry` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_selection_app::InferencePath` → `crate::model_selection_app` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_selection_app::ModelCommandGuard` → `crate::model_selection_app` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::model_selection_app` → `crate::model_selection_app` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::tests — `crates/whisper-core/src/application/runtime/tests.rs`

Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-core/src/application/runtime/tests.rs:1-4`.

Mesure : 0 production / 4 tests / 4 physiques. `crates/whisper-core/src/application/runtime/tests.rs:1-4`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/tests.rs:1-4`.

Fonctions avec corps : aucune relevée dans `crates/whisper-core/src/application/runtime/tests.rs:1-4`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/tests.rs:1-4`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/tests.rs:1-4`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application::runtime::unified — `crates/whisper-core/src/application/runtime/unified.rs`

Projette sessions unifiées, pages document/audio et commandes de capture. `crates/whisper-core/src/application/runtime/unified.rs:1-1128`

Mesure : 1128 production / 0 tests / 1128 physiques. `crates/whisper-core/src/application/runtime/unified.rs:1-1128`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application/runtime/unified.rs:1-1128`.

Fonctions avec corps : `add_unified_session_card` `crates/whisper-core/src/application/runtime/unified.rs:4-77`; `project_unified_import_event` `crates/whisper-core/src/application/runtime/unified.rs:79-162`; `handle_unified_capture_event` `crates/whisper-core/src/application/runtime/unified.rs:164-235`; `handle_unified_capture_progress` `crates/whisper-core/src/application/runtime/unified.rs:237-295`; `handle_unified_capture_text` `crates/whisper-core/src/application/runtime/unified.rs:297-368`; `handle_unified_prepared_segment` `crates/whisper-core/src/application/runtime/unified.rs:370-448`; `handle_unified_persist_end` `crates/whisper-core/src/application/runtime/unified.rs:450-517`; `handle_unified_document_page` `crates/whisper-core/src/application/runtime/unified.rs:519-579`; `handle_unified_audio_page` `crates/whisper-core/src/application/runtime/unified.rs:581-663`; `handle_unified_artifacts` `crates/whisper-core/src/application/runtime/unified.rs:665-692`; `handle_unified_terminal` `crates/whisper-core/src/application/runtime/unified.rs:694-751`; `retry_pending_unified_capture_segments` `crates/whisper-core/src/application/runtime/unified.rs:753-761`; `retry_unified_capture_segment` `crates/whisper-core/src/application/runtime/unified.rs:763-792`; `maybe_publish_unified_capture` `crates/whisper-core/src/application/runtime/unified.rs:794-823`; `handle_unified_selection_command` `crates/whisper-core/src/application/runtime/unified.rs:825-841`; `select_unified_session` `crates/whisper-core/src/application/runtime/unified.rs:843-870`; `read_unified_document` `crates/whisper-core/src/application/runtime/unified.rs:872-918`; `read_unified_audio` `crates/whisper-core/src/application/runtime/unified.rs:920-977`; `start_unified_live_command` `crates/whisper-core/src/application/runtime/unified.rs:979-1090`; `stop_unified_live_command` `crates/whisper-core/src/application/runtime/unified.rs:1092-1127`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/src/application/runtime/unified.rs:1-1128`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application/runtime/unified.rs:1-1128`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::application — `crates/whisper-core/src/application.rs`

Réexporte contrats et ImportApplication depuis ses modules privés. `crates/whisper-core/src/application.rs:1-9`

Mesure : 9 production / 0 tests / 9 physiques. `crates/whisper-core/src/application.rs:1-9`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/application.rs:1-9`.

Fonctions avec corps : aucune relevée dans `crates/whisper-core/src/application.rs:1-9`.

Déclarations `pub` (visibilité sous conditions) : `pub use contracts::{` `crates/whisper-core/src/application.rs:5`; `pub use runtime::ImportApplication;` `crates/whisper-core/src/application.rs:9`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/application.rs:1-9`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::audio — `crates/whisper-core/src/audio.rs`

Reconnaît les fins audio courtes et silencieuses par calcul d’énergie. `crates/whisper-core/src/audio.rs:1-45`

Mesure : 26 production / 19 tests / 45 physiques. `crates/whisper-core/src/audio.rs:1-45`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/audio.rs:1-45`.

Fonctions avec corps : `is_quiet_short_tail` `crates/whisper-core/src/audio.rs:4-25`; `almost_silent_file_end_produces_no_text` [tests] `crates/whisper-core/src/audio.rs:32-36`; `short_word_after_silence_is_kept_for_transcription` [tests] `crates/whisper-core/src/audio.rs:39-44`.

Déclarations `pub` (visibilité sous conditions) : `pub fn is_quiet_short_tail(pcm: &[f32]) -> bool {` `crates/whisper-core/src/audio.rs:4`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/audio.rs:1-45`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `almost_silent_file_end_produces_no_text` `crates/whisper-core/src/audio.rs:32-36`, `short_word_after_silence_is_kept_for_transcription` `crates/whisper-core/src/audio.rs:39-44`. `crates/whisper-core/src/audio.rs:27-45`

Mesure locale : 0 production / 19 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/audio.rs:27-45`

Types locaux : aucun relevé dans `crates/whisper-core/src/audio.rs:27-45`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/audio.rs:27-45`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::compute_policy — `crates/whisper-core/src/compute_policy.rs`

Décide le backend initial et les suites permises après son échec. `crates/whisper-core/src/compute_policy.rs:1-67`

Mesure : 67 production / 0 tests / 67 physiques. `crates/whisper-core/src/compute_policy.rs:1-67`

Types : `BackendAttempt` (enum, hors tests) `crates/whisper-core/src/compute_policy.rs:5-8`; `BackendDecision` (enum, hors tests) `crates/whisper-core/src/compute_policy.rs:12-17`; `ComputePolicyError` (enum, hors tests) `crates/whisper-core/src/compute_policy.rs:20-24`; `ComputePolicy` (struct, hors tests) `crates/whisper-core/src/compute_policy.rs:27`.

Fonctions avec corps : `initial` `crates/whisper-core/src/compute_policy.rs:30-41`; `after_failure` `crates/whisper-core/src/compute_policy.rs:44-55`; `validate_fallback` `crates/whisper-core/src/compute_policy.rs:57-66`.

Déclarations `pub` (visibilité sous conditions) : `pub enum BackendAttempt {` `crates/whisper-core/src/compute_policy.rs:5`; `pub enum BackendDecision {` `crates/whisper-core/src/compute_policy.rs:12`; `pub enum ComputePolicyError {` `crates/whisper-core/src/compute_policy.rs:20`; `pub struct ComputePolicy;` `crates/whisper-core/src/compute_policy.rs:27`; `pub fn initial(` `crates/whisper-core/src/compute_policy.rs:30`; `pub fn after_failure(` `crates/whisper-core/src/compute_policy.rs:44`; `pub fn validate_fallback(` `crates/whisper-core/src/compute_policy.rs:57`.

Dépend de, via `use crate::` : hors tests : `crate::domain::ComputeChoice` → `crate::domain` `crates/whisper-core/src/compute_policy.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::domain — `crates/whisper-core/src/domain.rs`

Définit identités, générations, plages audio et transitions JobState. `crates/whisper-core/src/domain.rs:1-170`

Mesure : 122 production / 48 tests / 170 physiques. `crates/whisper-core/src/domain.rs:1-170`

Types : `JobId` (struct, hors tests) `crates/whisper-core/src/domain.rs:5`; `Generation` (struct, hors tests) `crates/whisper-core/src/domain.rs:8`; `SegmentId` (struct, hors tests) `crates/whisper-core/src/domain.rs:28`; `SourceRange` (struct, hors tests) `crates/whisper-core/src/domain.rs:31-35`; `JobConfig` (struct, hors tests) `crates/whisper-core/src/domain.rs:48-51`; `LanguageChoice` (enum, hors tests) `crates/whisper-core/src/domain.rs:54-57`; `ComputeChoice` (enum, hors tests) `crates/whisper-core/src/domain.rs:60-64`; `JobState` (enum, hors tests) `crates/whisper-core/src/domain.rs:67-80`; `TransitionError` (enum, hors tests) `crates/whisper-core/src/domain.rs:83-85`.

Fonctions avec corps : `first` `crates/whisper-core/src/domain.rs:11-13`; `get` `crates/whisper-core/src/domain.rs:15-17`; `next` `crates/whisper-core/src/domain.rs:19-24`; `new` `crates/whisper-core/src/domain.rs:38-44`; `transition` `crates/whisper-core/src/domain.rs:88-120`; `generation_is_positive_and_monotonic` [tests] `crates/whisper-core/src/domain.rs:128-132`; `source_range_rejects_empty_or_unclocked_ranges` [tests] `crates/whisper-core/src/domain.rs:135-139`; `live_lifecycle_requires_drain_before_finalization` [tests] `crates/whisper-core/src/domain.rs:142-156`; `interrupted_import_needs_explicit_return_to_queue` [tests] `crates/whisper-core/src/domain.rs:159-169`.

Déclarations `pub` (visibilité sous conditions) : `pub struct JobId(pub u128);` `crates/whisper-core/src/domain.rs:5`; `pub struct Generation(NonZeroU64);` `crates/whisper-core/src/domain.rs:8`; `pub fn first() -> Self {` `crates/whisper-core/src/domain.rs:11`; `pub fn get(self) -> u64 {` `crates/whisper-core/src/domain.rs:15`; `pub fn next(self) -> Option<Self> {` `crates/whisper-core/src/domain.rs:19`; `pub struct SegmentId(pub u64);` `crates/whisper-core/src/domain.rs:28`; `pub struct SourceRange {` `crates/whisper-core/src/domain.rs:31`; `pub fn new(start_sample: u64, end_sample: u64, sample_rate_hz: u32) -> Option<Self> {` `crates/whisper-core/src/domain.rs:38`; `pub struct JobConfig {` `crates/whisper-core/src/domain.rs:48`; `pub enum LanguageChoice {` `crates/whisper-core/src/domain.rs:54`; `pub enum ComputeChoice {` `crates/whisper-core/src/domain.rs:60`; `pub enum JobState {` `crates/whisper-core/src/domain.rs:67`; `pub enum TransitionError {` `crates/whisper-core/src/domain.rs:83`; `pub fn transition(self, to: Self) -> Result<Self, TransitionError> {` `crates/whisper-core/src/domain.rs:88`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/domain.rs:1-170`.

Utilisé par, via ces imports : hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:1`<br>hors tests : `crate::application::event_metadata` `crates/whisper-core/src/application/event_metadata.rs:1`<br>hors tests : `crate::application::event_metadata` `crates/whisper-core/src/application/event_metadata.rs:1`<br>hors tests : `crate::application::event_metadata` `crates/whisper-core/src/application/event_metadata.rs:1`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:1`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:2`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:2`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:2`<br>hors tests : `crate::compute_policy` `crates/whisper-core/src/compute_policy.rs:1`<br>hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:133`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::ports` `crates/whisper-core/src/ports.rs:1`.

#### Inline `tests`

Tests/auxiliaires : `generation_is_positive_and_monotonic` `crates/whisper-core/src/domain.rs:128-132`, `source_range_rejects_empty_or_unclocked_ranges` `crates/whisper-core/src/domain.rs:135-139`, `live_lifecycle_requires_drain_before_finalization` `crates/whisper-core/src/domain.rs:142-156`, `interrupted_import_needs_explicit_return_to_queue` `crates/whisper-core/src/domain.rs:159-169`. `crates/whisper-core/src/domain.rs:123-170`

Mesure locale : 0 production / 48 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/domain.rs:123-170`

Types locaux : aucun relevé dans `crates/whisper-core/src/domain.rs:123-170`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/domain.rs:123-170`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::engine_capabilities — `crates/whisper-core/src/engine_capabilities.rs`

Projette et valide les combinaisons moteurs, modèles et capacités. `crates/whisper-core/src/engine_capabilities.rs:1-527`

Mesure : 527 production / 0 tests / 527 physiques. `crates/whisper-core/src/engine_capabilities.rs:1-527`

Types : `Support` (enum, hors tests) `crates/whisper-core/src/engine_capabilities.rs:6-11`; `Qualification` (enum, hors tests) `crates/whisper-core/src/engine_capabilities.rs:14-20`; `Capability` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:23-26`; `LanguageCode` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:49`; `LanguageCapability` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:52-56`; `ResourceKind` (enum, hors tests) `crates/whisper-core/src/engine_capabilities.rs:59-63`; `CapabilityTarget` (enum, hors tests) `crates/whisper-core/src/engine_capabilities.rs:66-71`; `ResourceRequirement` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:74-79`; `ResourceObservation` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:82-86`; `ConfigurationCapabilities` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:89-95`; `EngineSnapshot` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:98-105`; `LanguageChoice` (enum, hors tests) `crates/whisper-core/src/engine_capabilities.rs:108-111`; `RefusalCause` (enum, hors tests) `crates/whisper-core/src/engine_capabilities.rs:114-126`; `LanguageOption` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:129-133`; `ConfigurationOption` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:136-143`; `ModelOption` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:146-153`; `EngineProjection` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:156-161`; `InferenceRequest` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:164-169`; `Admission` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:172-176`; `Refusal` (struct, hors tests) `crates/whisper-core/src/engine_capabilities.rs:179-181`.

Fonctions avec corps : `is_qualified` `crates/whisper-core/src/engine_capabilities.rs:34-36`; `is_admissible` `crates/whisper-core/src/engine_capabilities.rs:39-45`; `project` `crates/whisper-core/src/engine_capabilities.rs:184-309`; `validate` `crates/whisper-core/src/engine_capabilities.rs:312-371`; `project_with_existing_recognized` `crates/whisper-core/src/engine_capabilities.rs:375-382`; `validate_with_existing_recognized` `crates/whisper-core/src/engine_capabilities.rs:385-393`; `promote_existing_recognized` `crates/whisper-core/src/engine_capabilities.rs:395-451`; `find_capability` `crates/whisper-core/src/engine_capabilities.rs:453-458`; `model_status_cause` `crates/whisper-core/src/engine_capabilities.rs:460-467`; `configuration_status` `crates/whisper-core/src/engine_capabilities.rs:469-485`; `configuration_cause` `crates/whisper-core/src/engine_capabilities.rs:487-515`; `capability_cause` `crates/whisper-core/src/engine_capabilities.rs:517-523`; `refuse` `crates/whisper-core/src/engine_capabilities.rs:525-527`.

Déclarations `pub` (visibilité sous conditions) : `pub enum Support {` `crates/whisper-core/src/engine_capabilities.rs:6`; `pub enum Qualification {` `crates/whisper-core/src/engine_capabilities.rs:14`; `pub struct Capability {` `crates/whisper-core/src/engine_capabilities.rs:23`; `pub const QUALIFIED: Self = Self {` `crates/whisper-core/src/engine_capabilities.rs:29`; `pub fn is_admissible(self) -> bool {` `crates/whisper-core/src/engine_capabilities.rs:39`; `pub struct LanguageCode(pub String);` `crates/whisper-core/src/engine_capabilities.rs:49`; `pub struct LanguageCapability {` `crates/whisper-core/src/engine_capabilities.rs:52`; `pub enum ResourceKind {` `crates/whisper-core/src/engine_capabilities.rs:59`; `pub enum CapabilityTarget {` `crates/whisper-core/src/engine_capabilities.rs:66`; `pub struct ResourceRequirement {` `crates/whisper-core/src/engine_capabilities.rs:74`; `pub struct ResourceObservation {` `crates/whisper-core/src/engine_capabilities.rs:82`; `pub struct ConfigurationCapabilities {` `crates/whisper-core/src/engine_capabilities.rs:89`; `pub struct EngineSnapshot {` `crates/whisper-core/src/engine_capabilities.rs:98`; `pub enum LanguageChoice {` `crates/whisper-core/src/engine_capabilities.rs:108`; `pub enum RefusalCause {` `crates/whisper-core/src/engine_capabilities.rs:114`; `pub struct LanguageOption {` `crates/whisper-core/src/engine_capabilities.rs:129`; `pub struct ConfigurationOption {` `crates/whisper-core/src/engine_capabilities.rs:136`; `pub struct ModelOption {` `crates/whisper-core/src/engine_capabilities.rs:146`; `pub struct EngineProjection {` `crates/whisper-core/src/engine_capabilities.rs:156`; `pub struct InferenceRequest {` `crates/whisper-core/src/engine_capabilities.rs:164`; `pub struct Admission {` `crates/whisper-core/src/engine_capabilities.rs:172`; `pub struct Refusal {` `crates/whisper-core/src/engine_capabilities.rs:179`; `pub fn project(` `crates/whisper-core/src/engine_capabilities.rs:184`; `pub fn validate(` `crates/whisper-core/src/engine_capabilities.rs:312`; `pub fn project_with_existing_recognized(` `crates/whisper-core/src/engine_capabilities.rs:375`; `pub fn validate_with_existing_recognized(` `crates/whisper-core/src/engine_capabilities.rs:385`.

Dépend de, via `use crate::` : hors tests : `crate::model_catalog::Backend` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::model_catalog::ConfigurationKey` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::model_catalog::ModelAvailability` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::model_catalog::ModelDefinition` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::model_catalog::ModelId` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::model_catalog` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::model_catalog::Backend` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_catalog::ModelId` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_catalog` → `crate::model_catalog` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_registry::TURBO_LANGUAGE_INPUTS` → `crate::model_registry` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_registry::configuration_key` → `crate::model_registry` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_registry::model` → `crate::model_registry` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_registry` → `crate::model_registry` `crates/whisper-core/src/engine_capabilities.rs:396`.

Utilisé par, via ces imports : hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`.

### crate::gpu_inventory — `crates/whisper-core/src/gpu_inventory.rs`

Qualifie et normalise observations et identités GPU. `crates/whisper-core/src/gpu_inventory.rs:1-341`

Mesure : 341 production / 0 tests / 341 physiques. `crates/whisper-core/src/gpu_inventory.rs:1-341`

Types : `GpuPresence` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:4-8`; `GpuRankSource` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:11-13`; `GpuWindowsRank` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:16-19`; `GpuIdentityMatch` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:22-27`; `GpuSupport` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:30-34`; `GpuAvailability` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:37-41`; `GpuState` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:44-49`; `GpuObservation` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:52-57`; `GpuDevice` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:60-64`; `Coverage` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:67-70`; `GpuInventory` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:73-77`; `GpuQualifiedObservation` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:81-88`; `GpuQualifiedDevice` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:91-98`; `GpuRuntimeIdentity` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:103-111`; `GpuCause` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:114-124`; `GpuCatalogueCoverage` (enum, hors tests) `crates/whisper-core/src/gpu_inventory.rs:127-131`; `GpuQualifiedCatalogue` (struct, hors tests) `crates/whisper-core/src/gpu_inventory.rs:134-138`.

Fonctions avec corps : `qualify` `crates/whisper-core/src/gpu_inventory.rs:140-257`; `normalize` `crates/whisper-core/src/gpu_inventory.rs:259-329`; `consistent` `crates/whisper-core/src/gpu_inventory.rs:331-341`.

Déclarations `pub` (visibilité sous conditions) : `pub enum GpuPresence {` `crates/whisper-core/src/gpu_inventory.rs:4`; `pub enum GpuRankSource {` `crates/whisper-core/src/gpu_inventory.rs:11`; `pub struct GpuWindowsRank {` `crates/whisper-core/src/gpu_inventory.rs:16`; `pub enum GpuIdentityMatch {` `crates/whisper-core/src/gpu_inventory.rs:22`; `pub enum GpuSupport {` `crates/whisper-core/src/gpu_inventory.rs:30`; `pub enum GpuAvailability {` `crates/whisper-core/src/gpu_inventory.rs:37`; `pub enum GpuState {` `crates/whisper-core/src/gpu_inventory.rs:44`; `pub struct GpuObservation {` `crates/whisper-core/src/gpu_inventory.rs:52`; `pub struct GpuDevice {` `crates/whisper-core/src/gpu_inventory.rs:60`; `pub enum Coverage {` `crates/whisper-core/src/gpu_inventory.rs:67`; `pub struct GpuInventory {` `crates/whisper-core/src/gpu_inventory.rs:73`; `pub struct GpuQualifiedObservation {` `crates/whisper-core/src/gpu_inventory.rs:81`; `pub struct GpuQualifiedDevice {` `crates/whisper-core/src/gpu_inventory.rs:91`; `pub struct GpuRuntimeIdentity {` `crates/whisper-core/src/gpu_inventory.rs:103`; `pub enum GpuCause {` `crates/whisper-core/src/gpu_inventory.rs:114`; `pub enum GpuCatalogueCoverage {` `crates/whisper-core/src/gpu_inventory.rs:127`; `pub struct GpuQualifiedCatalogue {` `crates/whisper-core/src/gpu_inventory.rs:134`; `pub fn qualify(` `crates/whisper-core/src/gpu_inventory.rs:140`; `pub fn normalize(` `crates/whisper-core/src/gpu_inventory.rs:259`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/gpu_inventory.rs:1-341`.

Utilisé par, via ces imports : hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`.

### crate::gpu_inventory_app — `crates/whisper-core/src/gpu_inventory_app.rs`

Suit une collecte GPU via un port et expose son état et sa sélection. `crates/whisper-core/src/gpu_inventory_app.rs:1-248`

Mesure : 248 production / 0 tests / 248 physiques. `crates/whisper-core/src/gpu_inventory_app.rs:1-248`

Types : `GpuInventoryPort` (trait, hors tests) `crates/whisper-core/src/gpu_inventory_app.rs:6-23`; `GpuCollection` (struct, hors tests) `crates/whisper-core/src/gpu_inventory_app.rs:26-29`; `GpuQualifiedCollection` (struct, hors tests) `crates/whisper-core/src/gpu_inventory_app.rs:32-35`; `GpuQualifiedInventoryState` (enum, hors tests) `crates/whisper-core/src/gpu_inventory_app.rs:38-44`; `GpuInventoryState` (enum, hors tests) `crates/whisper-core/src/gpu_inventory_app.rs:62-68`; `GpuInventoryApp` (struct, hors tests) `crates/whisper-core/src/gpu_inventory_app.rs:70-78`.

Fonctions avec corps : `uses_qualified_catalogue` `crates/whisper-core/src/gpu_inventory_app.rs:10-12`; `poll_qualified` `crates/whisper-core/src/gpu_inventory_app.rs:14-16`; `poll_runtime_identities` `crates/whisper-core/src/gpu_inventory_app.rs:18-22`; `from_collection` `crates/whisper-core/src/gpu_inventory_app.rs:47-58`; `new` `crates/whisper-core/src/gpu_inventory_app.rs:81-93`; `start` `crates/whisper-core/src/gpu_inventory_app.rs:95-121`; `poll` `crates/whisper-core/src/gpu_inventory_app.rs:123-182`; `close` `crates/whisper-core/src/gpu_inventory_app.rs:184-190`; `state` `crates/whisper-core/src/gpu_inventory_app.rs:192-194`; `qualified_state` `crates/whisper-core/src/gpu_inventory_app.rs:196-198`; `runtime_identities` `crates/whisper-core/src/gpu_inventory_app.rs:202-204`; `resolve_selection` `crates/whisper-core/src/gpu_inventory_app.rs:206-212`; `legacy_view` `crates/whisper-core/src/gpu_inventory_app.rs:215-248`.

Déclarations `pub` (visibilité sous conditions) : `pub trait GpuInventoryPort {` `crates/whisper-core/src/gpu_inventory_app.rs:6`; `pub struct GpuCollection {` `crates/whisper-core/src/gpu_inventory_app.rs:26`; `pub struct GpuQualifiedCollection {` `crates/whisper-core/src/gpu_inventory_app.rs:32`; `pub enum GpuQualifiedInventoryState {` `crates/whisper-core/src/gpu_inventory_app.rs:38`; `pub enum GpuInventoryState {` `crates/whisper-core/src/gpu_inventory_app.rs:62`; `pub struct GpuInventoryApp {` `crates/whisper-core/src/gpu_inventory_app.rs:70`; `pub fn new(port: Box<dyn GpuInventoryPort>) -> Self {` `crates/whisper-core/src/gpu_inventory_app.rs:81`; `pub fn start(&mut self) {` `crates/whisper-core/src/gpu_inventory_app.rs:95`; `pub fn poll(&mut self) {` `crates/whisper-core/src/gpu_inventory_app.rs:123`; `pub fn close(&mut self) {` `crates/whisper-core/src/gpu_inventory_app.rs:184`; `pub fn state(&self) -> &GpuInventoryState {` `crates/whisper-core/src/gpu_inventory_app.rs:192`; `pub fn qualified_state(&self) -> &GpuQualifiedInventoryState {` `crates/whisper-core/src/gpu_inventory_app.rs:196`; `pub fn runtime_identities(&self) -> &[GpuRuntimeIdentity] {` `crates/whisper-core/src/gpu_inventory_app.rs:202`; `pub fn resolve_selection(` `crates/whisper-core/src/gpu_inventory_app.rs:206`.

Dépend de, via `use crate::` : hors tests : `crate::gpu_inventory::GpuCatalogueCoverage` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::GpuInventory` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::GpuObservation` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::GpuQualifiedCatalogue` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::GpuQualifiedObservation` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::GpuRuntimeIdentity` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::normalize` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory::qualify` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`<br>hors tests : `crate::gpu_inventory` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_inventory_app.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::gpu_selection` `crates/whisper-core/src/gpu_selection.rs:1`.

### crate::gpu_selection — `crates/whisper-core/src/gpu_selection.rs`

Résout les préférences GPU et vérifie les attestations du candidat. `crates/whisper-core/src/gpu_selection.rs:1-204`

Mesure : 204 production / 0 tests / 204 physiques. `crates/whisper-core/src/gpu_selection.rs:1-204`

Types : `SelectionReason` (enum, hors tests) `crates/whisper-core/src/gpu_selection.rs:8-16`; `GpuCandidate` (struct, hors tests) `crates/whisper-core/src/gpu_selection.rs:19-23`; `SessionCandidate` (enum, hors tests) `crates/whisper-core/src/gpu_selection.rs:26-29`; `SelectionResolution` (enum, hors tests) `crates/whisper-core/src/gpu_selection.rs:32-37`; `EffectiveEngine` (enum, hors tests) `crates/whisper-core/src/gpu_selection.rs:40-43`; `GpuSelectionEvent` (enum, hors tests) `crates/whisper-core/src/gpu_selection.rs:46-54`; `AttestedBackend` (enum, hors tests) `crates/whisper-core/src/gpu_selection.rs:57-60`.

Fonctions avec corps : `resolve` `crates/whisper-core/src/gpu_selection.rs:62-143`; `attest` `crates/whisper-core/src/gpu_selection.rs:145-164`; `candidate_event` `crates/whisper-core/src/gpu_selection.rs:166-171`; `attestation_event` `crates/whisper-core/src/gpu_selection.rs:173-180`; `is_compatible` `crates/whisper-core/src/gpu_selection.rs:182-188`; `candidate` `crates/whisper-core/src/gpu_selection.rs:190-196`; `fallback_or_refuse` `crates/whisper-core/src/gpu_selection.rs:198-204`.

Déclarations `pub` (visibilité sous conditions) : `pub enum SelectionReason {` `crates/whisper-core/src/gpu_selection.rs:8`; `pub struct GpuCandidate {` `crates/whisper-core/src/gpu_selection.rs:19`; `pub enum SessionCandidate {` `crates/whisper-core/src/gpu_selection.rs:26`; `pub enum SelectionResolution {` `crates/whisper-core/src/gpu_selection.rs:32`; `pub enum EffectiveEngine {` `crates/whisper-core/src/gpu_selection.rs:40`; `pub enum GpuSelectionEvent {` `crates/whisper-core/src/gpu_selection.rs:46`; `pub enum AttestedBackend<'a> {` `crates/whisper-core/src/gpu_selection.rs:57`; `pub fn resolve(` `crates/whisper-core/src/gpu_selection.rs:62`; `pub fn attest(` `crates/whisper-core/src/gpu_selection.rs:145`; `pub fn candidate_event(session_id: u64, candidate: SessionCandidate) -> GpuSelectionEvent {` `crates/whisper-core/src/gpu_selection.rs:166`; `pub fn attestation_event(` `crates/whisper-core/src/gpu_selection.rs:173`.

Dépend de, via `use crate::` : hors tests : `crate::domain::ComputeChoice` → `crate::domain` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_inventory::GpuAvailability` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_inventory::GpuIdentityMatch` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_inventory::GpuQualifiedDevice` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_inventory::GpuSupport` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_inventory` → `crate::gpu_inventory` `crates/whisper-core/src/gpu_selection.rs:1`<br>hors tests : `crate::gpu_inventory_app::GpuQualifiedInventoryState` → `crate::gpu_inventory_app` `crates/whisper-core/src/gpu_selection.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc — `crates/whisper-core/src/ipc.rs`

Définit enveloppes, commandes et événements du protocole Turbo. `crates/whisper-core/src/ipc.rs:1-343`

Mesure : 326 production / 17 tests / 343 physiques. `crates/whisper-core/src/ipc.rs:1-343`

Types : `IpcEnvelope` (struct, hors tests) `crates/whisper-core/src/ipc.rs:8-11`; `WorkerCommand` (enum, hors tests) `crates/whisper-core/src/ipc.rs:27-146`; `WorkerEvent` (enum, hors tests) `crates/whisper-core/src/ipc.rs:149-234`; `GpuQualificationTarget` (struct, hors tests) `crates/whisper-core/src/ipc.rs:237-246`; `GpuQualificationResult` (struct, hors tests) `crates/whisper-core/src/ipc.rs:249-260`; `BackendKind` (enum, hors tests) `crates/whisper-core/src/ipc.rs:263-266`; `WorkerErrorCode` (enum, hors tests) `crates/whisper-core/src/ipc.rs:269-289`; `WorkerSegmentDto` (struct, hors tests) `crates/whisper-core/src/ipc.rs:292-299`.

Fonctions avec corps : `new` `crates/whisper-core/src/ipc.rs:14-19`; `is_supported` `crates/whisper-core/src/ipc.rs:21-23`; `from` `crates/whisper-core/src/ipc.rs:302-311`; `from` `crates/whisper-core/src/ipc.rs:315-324`; `protocol_version_is_explicit_and_checked` [tests] `crates/whisper-core/src/ipc.rs:332-342`.

Déclarations `pub` (visibilité sous conditions) : `pub const IPC_PROTOCOL_VERSION: u16 = 2;` `crates/whisper-core/src/ipc.rs:5`; `pub struct IpcEnvelope<T> {` `crates/whisper-core/src/ipc.rs:8`; `pub fn new(message: T) -> Self {` `crates/whisper-core/src/ipc.rs:14`; `pub fn is_supported(&self) -> bool {` `crates/whisper-core/src/ipc.rs:21`; `pub enum WorkerCommand {` `crates/whisper-core/src/ipc.rs:27`; `pub enum WorkerEvent {` `crates/whisper-core/src/ipc.rs:149`; `pub struct GpuQualificationTarget {` `crates/whisper-core/src/ipc.rs:237`; `pub struct GpuQualificationResult {` `crates/whisper-core/src/ipc.rs:249`; `pub enum BackendKind {` `crates/whisper-core/src/ipc.rs:263`; `pub enum WorkerErrorCode {` `crates/whisper-core/src/ipc.rs:269`; `pub struct WorkerSegmentDto {` `crates/whisper-core/src/ipc.rs:292`.

Dépend de, via `use crate::` : hors tests : `crate::domain::Generation` → `crate::domain` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::domain::JobConfig` → `crate::domain` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::domain::JobId` → `crate::domain` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::domain::SegmentId` → `crate::domain` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::domain::SourceRange` → `crate::domain` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/ipc.rs:1`<br>hors tests : `crate::ports::DecodeRequest` → `crate::ports` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ports::DecodedPcmBlock` → `crate::ports` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ports::PortError` → `crate::ports` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ports::WorkerSegment` → `crate::ports` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ports` → `crate::ports` `crates/whisper-core/src/ipc.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `protocol_version_is_explicit_and_checked` `crates/whisper-core/src/ipc.rs:332-342`. `crates/whisper-core/src/ipc.rs:327-343`

Mesure locale : 0 production / 17 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/ipc.rs:327-343`

Types locaux : aucun relevé dans `crates/whisper-core/src/ipc.rs:327-343`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/ipc.rs:327-343`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate — `crates/whisper-core/src/lib.rs`

Déclare les modules et interfaces de bibliothèque. `crates/whisper-core/src/lib.rs:1-45`

Mesure : 45 production / 0 tests / 45 physiques. `crates/whisper-core/src/lib.rs:1-45`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/src/lib.rs:1-45`.

Fonctions avec corps : aucune relevée dans `crates/whisper-core/src/lib.rs:1-45`.

Déclarations `pub` (visibilité sous conditions) : `pub mod application;` `crates/whisper-core/src/lib.rs:3`; `pub mod audio;` `crates/whisper-core/src/lib.rs:4`; `pub mod compute_policy;` `crates/whisper-core/src/lib.rs:5`; `pub mod domain;` `crates/whisper-core/src/lib.rs:6`; `pub mod engine_capabilities;` `crates/whisper-core/src/lib.rs:7`; `pub mod gpu_inventory;` `crates/whisper-core/src/lib.rs:8`; `pub mod gpu_inventory_app;` `crates/whisper-core/src/lib.rs:9`; `pub mod gpu_selection;` `crates/whisper-core/src/lib.rs:10`; `pub mod ipc;` `crates/whisper-core/src/lib.rs:11`; `pub mod microphone;` `crates/whisper-core/src/lib.rs:12`; `pub mod microphone_level;` `crates/whisper-core/src/lib.rs:13`; `pub mod model_catalog;` `crates/whisper-core/src/lib.rs:14`; `pub mod model_registry;` `crates/whisper-core/src/lib.rs:15`; `pub mod model_selection_app;` `crates/whisper-core/src/lib.rs:16`; `pub mod parakeet_ipc;` `crates/whisper-core/src/lib.rs:17`; `pub mod parakeet_scheduler;` `crates/whisper-core/src/lib.rs:18`; `pub mod parakeet_session;` `crates/whisper-core/src/lib.rs:19`; `pub mod parakeet_windows;` `crates/whisper-core/src/lib.rs:20`; `pub mod path_settings;` `crates/whisper-core/src/lib.rs:21`; `pub mod ports;` `crates/whisper-core/src/lib.rs:22`; `pub use application::{` `crates/whisper-core/src/lib.rs:24`; `pub use compute_policy::{BackendAttempt, BackendDecision, ComputePolicy, ComputePolicyError};` `crates/whisper-core/src/lib.rs:28`; `pub use domain::{` `crates/whisper-core/src/lib.rs:29`; `pub use microphone::{` `crates/whisper-core/src/lib.rs:33`; `pub use microphone_level::{` `crates/whisper-core/src/lib.rs:36`; `pub use ports::{` `crates/whisper-core/src/lib.rs:40`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/lib.rs:1-45`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::microphone — `crates/whisper-core/src/microphone.rs`

Définit le port microphone et conserve choix préféré/effectif et notifications. `crates/whisper-core/src/microphone.rs:1-247`

Mesure : 161 production / 86 tests / 247 physiques. `crates/whisper-core/src/microphone.rs:1-247`

Types : `MicrophoneChoice` (enum, hors tests) `crates/whisper-core/src/microphone.rs:2-6`; `MicrophoneDevice` (struct, hors tests) `crates/whisper-core/src/microphone.rs:9-13`; `MicrophonePort` (trait, hors tests) `crates/whisper-core/src/microphone.rs:21-27`; `NullMicrophonePort` (struct, hors tests) `crates/whisper-core/src/microphone.rs:29`; `MicrophoneSettings` (struct, hors tests) `crates/whisper-core/src/microphone.rs:37-46`; `ChangingPort` (struct, tests) `crates/whisper-core/src/microphone.rs:167`; `FailingPort` (struct, tests) `crates/whisper-core/src/microphone.rs:169`.

Fonctions avec corps : `selectable` `crates/whisper-core/src/microphone.rs:16-18`; `default_input_identity` `crates/whisper-core/src/microphone.rs:24-26`; `list_inputs` `crates/whisper-core/src/microphone.rs:32-34`; `new` `crates/whisper-core/src/microphone.rs:49-74`; `refresh` `crates/whisper-core/src/microphone.rs:76-101`; `select` `crates/whisper-core/src/microphone.rs:103-116`; `preferred` `crates/whisper-core/src/microphone.rs:118-120`; `effective` `crates/whisper-core/src/microphone.rs:122-124`; `devices` `crates/whisper-core/src/microphone.rs:126-128`; `list_failed` `crates/whisper-core/src/microphone.rs:130-132`; `take_notice` `crates/whisper-core/src/microphone.rs:134-136`; `target_identity` `crates/whisper-core/src/microphone.rs:138-143`; `capture_choice` `crates/whisper-core/src/microphone.rs:145-149`; `target_error` `crates/whisper-core/src/microphone.rs:151-159`; `list_inputs` [tests] `crates/whisper-core/src/microphone.rs:172-174`; `list_inputs` [tests] `crates/whisper-core/src/microphone.rs:178-180`; `device` [tests] `crates/whisper-core/src/microphone.rs:183-189`; `preferred_id_survives_temporary_default_and_recovers_on_next_launch` [tests] `crates/whisper-core/src/microphone.rs:192-210`; `changed_id_requires_explicit_selection` [tests] `crates/whisper-core/src/microphone.rs:213-232`; `enumeration_failure_is_not_mistaken_for_missing_preferred_device` [tests] `crates/whisper-core/src/microphone.rs:235-246`.

Déclarations `pub` (visibilité sous conditions) : `pub enum MicrophoneChoice {` `crates/whisper-core/src/microphone.rs:2`; `pub struct MicrophoneDevice {` `crates/whisper-core/src/microphone.rs:9`; `pub fn selectable(&self) -> bool {` `crates/whisper-core/src/microphone.rs:16`; `pub trait MicrophonePort {` `crates/whisper-core/src/microphone.rs:21`; `pub struct NullMicrophonePort;` `crates/whisper-core/src/microphone.rs:29`; `pub struct MicrophoneSettings {` `crates/whisper-core/src/microphone.rs:37`; `pub fn new(port: Box<dyn MicrophonePort>, preferred: MicrophoneChoice) -> Self {` `crates/whisper-core/src/microphone.rs:49`; `pub fn refresh(&mut self) -> Result<(), String> {` `crates/whisper-core/src/microphone.rs:76`; `pub fn select(&mut self, choice: MicrophoneChoice) -> Result<(), String> {` `crates/whisper-core/src/microphone.rs:103`; `pub fn preferred(&self) -> &MicrophoneChoice {` `crates/whisper-core/src/microphone.rs:118`; `pub fn effective(&self) -> &MicrophoneChoice {` `crates/whisper-core/src/microphone.rs:122`; `pub fn devices(&self) -> &[MicrophoneDevice] {` `crates/whisper-core/src/microphone.rs:126`; `pub fn list_failed(&self) -> bool {` `crates/whisper-core/src/microphone.rs:130`; `pub fn take_notice(&mut self) -> Option<String> {` `crates/whisper-core/src/microphone.rs:134`; `pub fn target_identity(&self) -> Option<&str> {` `crates/whisper-core/src/microphone.rs:138`; `pub fn capture_choice(&self) -> MicrophoneChoice {` `crates/whisper-core/src/microphone.rs:145`; `pub fn target_error(&self) -> Option<&str> {` `crates/whisper-core/src/microphone.rs:151`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/microphone.rs:1-247`.

Utilisé par, via ces imports : hors tests : `crate::microphone_level` `crates/whisper-core/src/microphone_level.rs:1`<br>tests : `crate::microphone_level` `crates/whisper-core/src/microphone_level.rs:555`.

#### Inline `tests`

Tests/auxiliaires : `list_inputs` `crates/whisper-core/src/microphone.rs:172-174`, `list_inputs` `crates/whisper-core/src/microphone.rs:178-180`, `device` `crates/whisper-core/src/microphone.rs:183-189`, `preferred_id_survives_temporary_default_and_recovers_on_next_launch` `crates/whisper-core/src/microphone.rs:192-210`. `crates/whisper-core/src/microphone.rs:162-247`

Mesure locale : 0 production / 86 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/microphone.rs:162-247`

Types locaux : `ChangingPort` `crates/whisper-core/src/microphone.rs:167`, `FailingPort` `crates/whisper-core/src/microphone.rs:169`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/microphone.rs:162-247`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::microphone_level — `crates/whisper-core/src/microphone_level.rs`

Définit gain, identités/révisions, mesures de niveau et de spectre. `crates/whisper-core/src/microphone_level.rs:1-628`

Mesure : 548 production / 80 tests / 628 physiques. `crates/whisper-core/src/microphone_level.rs:1-628`

Types : `GainDb` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:23`; `GainError` (enum, hors tests) `crates/whisper-core/src/microphone_level.rs:46-48`; `MicrophoneIdentity` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:64`; `SelectionRevision` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:81`; `SelectionContext` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:96-100`; `GainRevision` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:138-141`; `MeasurementStatus` (enum, hors tests) `crates/whisper-core/src/microphone_level.rs:161-165`; `LevelMeasurement` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:168-173`; `MeterLevel` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:176-182`; `MicrophoneMeter` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:218-237`; `SpectrumBin` (enum, hors tests) `crates/whisper-core/src/microphone_level.rs:245-248`; `MicrophoneSpectrum` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:275-294`; `MicrophoneSpectrumUpdate` (enum, hors tests) `crates/whisper-core/src/microphone_level.rs:315-326`; `MicrophoneAcquisitionContext` (struct, hors tests) `crates/whisper-core/src/microphone_level.rs:340-350`; `MicrophoneLevelError` (enum, hors tests) `crates/whisper-core/src/microphone_level.rs:525-531`.

Fonctions avec corps : `monotonic_millis` `crates/whisper-core/src/microphone_level.rs:6-8`; `monotonic_millis_at` `crates/whisper-core/src/microphone_level.rs:10-17`; `new` `crates/whisper-core/src/microphone_level.rs:28-34`; `value` `crates/whisper-core/src/microphone_level.rs:36-38`; `linear_factor` `crates/whisper-core/src/microphone_level.rs:40-42`; `fmt` `crates/whisper-core/src/microphone_level.rs:51-58`; `new` `crates/whisper-core/src/microphone_level.rs:67-73`; `as_str` `crates/whisper-core/src/microphone_level.rs:75-77`; `new` `crates/whisper-core/src/microphone_level.rs:84-86`; `value` `crates/whisper-core/src/microphone_level.rs:88-90`; `new` `crates/whisper-core/src/microphone_level.rs:103-118`; `revision` `crates/whisper-core/src/microphone_level.rs:120-122`; `preferred` `crates/whisper-core/src/microphone_level.rs:124-126`; `target` `crates/whisper-core/src/microphone_level.rs:128-130`; `permits` `crates/whisper-core/src/microphone_level.rs:132-134`; `neutral` `crates/whisper-core/src/microphone_level.rs:144-149`; `next` `crates/whisper-core/src/microphone_level.rs:151-157`; `from` `crates/whisper-core/src/microphone_level.rs:185-199`; `scaled` `crates/whisper-core/src/microphone_level.rs:186-192`; `rms` `crates/whisper-core/src/microphone_level.rs:203-205`; `peak` `crates/whisper-core/src/microphone_level.rs:207-209`; `rms_dbfs` `crates/whisper-core/src/microphone_level.rs:211-214`; `from_power` `crates/whisper-core/src/microphone_level.rs:251-264`; `dbfs_per_hz` `crates/whisper-core/src/microphone_level.rs:266-271`; `is_valid` `crates/whisper-core/src/microphone_level.rs:297-311`; `owner` `crates/whisper-core/src/microphone_level.rs:329-336`; `new` `crates/whisper-core/src/microphone_level.rs:353-373`; `bind_source` `crates/whisper-core/src/microphone_level.rs:377-392`; `permits_source` `crates/whisper-core/src/microphone_level.rs:394-398`; `accept_spectrum_update` `crates/whisper-core/src/microphone_level.rs:402-478`; `absent` `crates/whisper-core/src/microphone_level.rs:482-489`; `from_samples` `crates/whisper-core/src/microphone_level.rs:491-516`; `rms_dbfs` `crates/whisper-core/src/microphone_level.rs:518-521`; `fmt` `crates/whisper-core/src/microphone_level.rs:534-544`; `gain_bounds_are_inclusive_and_invalid_values_are_rejected` [tests] `crates/whisper-core/src/microphone_level.rs:558-564`; `selection_context_targets_only_its_current_identity_and_revision` [tests] `crates/whisper-core/src/microphone_level.rs:567-594`; `silence_and_absent_measurement_are_distinct` [tests] `crates/whisper-core/src/microphone_level.rs:597-610`; `gain_revisions_are_monotonic_and_checked` [tests] `crates/whisper-core/src/microphone_level.rs:613-627`.

Déclarations `pub` (visibilité sous conditions) : `pub fn monotonic_millis() -> u64 {` `crates/whisper-core/src/microphone_level.rs:6`; `pub fn monotonic_millis_at(moment: Instant) -> u64 {` `crates/whisper-core/src/microphone_level.rs:10`; `pub const MIN_GAIN_DB: i8 = -24;` `crates/whisper-core/src/microphone_level.rs:19`; `pub const MAX_GAIN_DB: i8 = 24;` `crates/whisper-core/src/microphone_level.rs:20`; `pub struct GainDb(i8);` `crates/whisper-core/src/microphone_level.rs:23`; `pub const NEUTRAL: Self = Self(0);` `crates/whisper-core/src/microphone_level.rs:26`; `pub fn new(value: i8) -> Result<Self, GainError> {` `crates/whisper-core/src/microphone_level.rs:28`; `pub const fn value(self) -> i8 {` `crates/whisper-core/src/microphone_level.rs:36`; `pub fn linear_factor(self) -> f64 {` `crates/whisper-core/src/microphone_level.rs:40`; `pub enum GainError {` `crates/whisper-core/src/microphone_level.rs:46`; `pub struct MicrophoneIdentity(String);` `crates/whisper-core/src/microphone_level.rs:64`; `pub fn new(value: impl Into<String>) -> Result<Self, MicrophoneLevelError> {` `crates/whisper-core/src/microphone_level.rs:67`; `pub fn as_str(&self) -> &str {` `crates/whisper-core/src/microphone_level.rs:75`; `pub struct SelectionRevision(u64);` `crates/whisper-core/src/microphone_level.rs:81`; `pub const fn new(value: u64) -> Self {` `crates/whisper-core/src/microphone_level.rs:84`; `pub const fn value(self) -> u64 {` `crates/whisper-core/src/microphone_level.rs:88`; `pub struct SelectionContext {` `crates/whisper-core/src/microphone_level.rs:96`; `pub fn new(` `crates/whisper-core/src/microphone_level.rs:103`; `pub const fn revision(&self) -> SelectionRevision {` `crates/whisper-core/src/microphone_level.rs:120`; `pub fn preferred(&self) -> &MicrophoneChoice {` `crates/whisper-core/src/microphone_level.rs:124`; `pub fn target(&self) -> Option<&MicrophoneIdentity> {` `crates/whisper-core/src/microphone_level.rs:128`; `pub fn permits(&self, revision: SelectionRevision, identity: &MicrophoneIdentity) -> bool {` `crates/whisper-core/src/microphone_level.rs:132`; `pub struct GainRevision {` `crates/whisper-core/src/microphone_level.rs:138`; `pub const fn neutral() -> Self {` `crates/whisper-core/src/microphone_level.rs:144`; `pub fn next(self, gain: GainDb) -> Result<Self, MicrophoneLevelError> {` `crates/whisper-core/src/microphone_level.rs:151`; `pub enum MeasurementStatus {` `crates/whisper-core/src/microphone_level.rs:161`; `pub struct LevelMeasurement {` `crates/whisper-core/src/microphone_level.rs:168`; `pub struct MeterLevel {` `crates/whisper-core/src/microphone_level.rs:176`; `pub fn rms(&self) -> f64 {` `crates/whisper-core/src/microphone_level.rs:203`; `pub fn peak(&self) -> f64 {` `crates/whisper-core/src/microphone_level.rs:207`; `pub fn rms_dbfs(&self) -> Option<f64> {` `crates/whisper-core/src/microphone_level.rs:211`; `pub struct MicrophoneMeter {` `crates/whisper-core/src/microphone_level.rs:218`; `pub const MICROPHONE_SPECTRUM_FFT_SIZE: u64 = 2048;` `crates/whisper-core/src/microphone_level.rs:239`; `pub const MICROPHONE_SPECTRUM_BIN_COUNT: usize = MICROPHONE_SPECTRUM_FFT_SIZE as usize / 2 + 1;` `crates/whisper-core/src/microphone_level.rs:240`; `pub enum SpectrumBin {` `crates/whisper-core/src/microphone_level.rs:245`; `pub fn from_power(power_fs2_per_hz: f64) -> Option<Self> {` `crates/whisper-core/src/microphone_level.rs:251`; `pub fn dbfs_per_hz(self) -> Option<f64> {` `crates/whisper-core/src/microphone_level.rs:266`; `pub struct MicrophoneSpectrum {` `crates/whisper-core/src/microphone_level.rs:275`; `pub fn is_valid(&self) -> bool {` `crates/whisper-core/src/microphone_level.rs:297`; `pub enum MicrophoneSpectrumUpdate {` `crates/whisper-core/src/microphone_level.rs:315`; `pub fn owner(&self) -> (crate::domain::JobId, crate::domain::Generation) {` `crates/whisper-core/src/microphone_level.rs:329`; `pub struct MicrophoneAcquisitionContext {` `crates/whisper-core/src/microphone_level.rs:340`; `pub fn new(` `crates/whisper-core/src/microphone_level.rs:353`; `pub fn bind_source(&mut self, source_identity: &str) -> bool {` `crates/whisper-core/src/microphone_level.rs:377`; `pub fn permits_source(&self, source_identity: &str) -> bool {` `crates/whisper-core/src/microphone_level.rs:394`; `pub fn accept_spectrum_update(` `crates/whisper-core/src/microphone_level.rs:402`; `pub const fn absent() -> Self {` `crates/whisper-core/src/microphone_level.rs:482`; `pub fn from_samples(samples: &[f32]) -> Self {` `crates/whisper-core/src/microphone_level.rs:491`; `pub fn rms_dbfs(self) -> Option<f64> {` `crates/whisper-core/src/microphone_level.rs:518`; `pub enum MicrophoneLevelError {` `crates/whisper-core/src/microphone_level.rs:525`.

Dépend de, via `use crate::` : hors tests : `crate::microphone::MicrophoneChoice` → `crate::microphone` `crates/whisper-core/src/microphone_level.rs:1`<br>tests : `crate::microphone::MicrophoneChoice` → `crate::microphone` `crates/whisper-core/src/microphone_level.rs:555`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `gain_bounds_are_inclusive_and_invalid_values_are_rejected` `crates/whisper-core/src/microphone_level.rs:558-564`, `selection_context_targets_only_its_current_identity_and_revision` `crates/whisper-core/src/microphone_level.rs:567-594`, `silence_and_absent_measurement_are_distinct` `crates/whisper-core/src/microphone_level.rs:597-610`, `gain_revisions_are_monotonic_and_checked` `crates/whisper-core/src/microphone_level.rs:613-627`. `crates/whisper-core/src/microphone_level.rs:549-628`

Mesure locale : 0 production / 80 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/microphone_level.rs:549-628`

Types locaux : aucun relevé dans `crates/whisper-core/src/microphone_level.rs:549-628`.

Imports locaux : `crate::microphone::MicrophoneChoice` `crates/whisper-core/src/microphone_level.rs:555`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::model_catalog — `crates/whisper-core/src/model_catalog.rs`

Définit identités, disponibilités et observations de modèles. `crates/whisper-core/src/model_catalog.rs:1-50`

Mesure : 50 production / 0 tests / 50 physiques. `crates/whisper-core/src/model_catalog.rs:1-50`

Types : `ModelId` (enum, hors tests) `crates/whisper-core/src/model_catalog.rs:5-8`; `Backend` (enum, hors tests) `crates/whisper-core/src/model_catalog.rs:15-18`; `ConfigurationKey` (struct, hors tests) `crates/whisper-core/src/model_catalog.rs:22-34`; `ModelAvailability` (enum, hors tests) `crates/whisper-core/src/model_catalog.rs:37-42`; `ModelDefinition` (struct, hors tests) `crates/whisper-core/src/model_catalog.rs:45-50`.

Fonctions avec corps : aucune relevée dans `crates/whisper-core/src/model_catalog.rs:1-50`.

Déclarations `pub` (visibilité sous conditions) : `pub enum ModelId {` `crates/whisper-core/src/model_catalog.rs:5`; `pub const ALL: [Self; 2] = [Self::WhisperLargeV3Turbo, Self::ParakeetTdt06BV3];` `crates/whisper-core/src/model_catalog.rs:11`; `pub enum Backend {` `crates/whisper-core/src/model_catalog.rs:15`; `pub struct ConfigurationKey {` `crates/whisper-core/src/model_catalog.rs:22`; `pub enum ModelAvailability {` `crates/whisper-core/src/model_catalog.rs:37`; `pub struct ModelDefinition {` `crates/whisper-core/src/model_catalog.rs:45`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/model_catalog.rs:1-50`.

Utilisé par, via ces imports : tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:1`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_registry` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`.

### crate::model_registry — `crates/whisper-core/src/model_registry.rs`

Déclare les modèles et construit leurs clés de configuration. `crates/whisper-core/src/model_registry.rs:1-345`

Mesure : 345 production / 0 tests / 345 physiques. `crates/whisper-core/src/model_registry.rs:1-345`

Types : `AdmissionBasis` (enum, hors tests) `crates/whisper-core/src/model_registry.rs:13-16`; `ModelArtifact` (struct, hors tests) `crates/whisper-core/src/model_registry.rs:19-24`; `ModelRecord` (struct, hors tests) `crates/whisper-core/src/model_registry.rs:27-32`.

Fonctions avec corps : `model` `crates/whisper-core/src/model_registry.rs:188-193`; `engine_snapshot` `crates/whisper-core/src/model_registry.rs:195-265`; `configuration_key` `crates/whisper-core/src/model_registry.rs:267-311`; `personal_configuration_key` `crates/whisper-core/src/model_registry.rs:314-331`; `existing` `crates/whisper-core/src/model_registry.rs:333-338`; `unsupported` `crates/whisper-core/src/model_registry.rs:340-345`.

Déclarations `pub` (visibilité sous conditions) : `pub enum AdmissionBasis {` `crates/whisper-core/src/model_registry.rs:13`; `pub struct ModelArtifact {` `crates/whisper-core/src/model_registry.rs:19`; `pub struct ModelRecord {` `crates/whisper-core/src/model_registry.rs:27`; `pub const TURBO_EXISTING_PROVENANCE: &str =` `crates/whisper-core/src/model_registry.rs:34`; `pub const PARAKEET_MAX_SINGLE_PASS_DURATION_MS: u64 = 30 * 60 * 1_000;` `crates/whisper-core/src/model_registry.rs:38`; `pub const PARAKEET_TEST_GPU_PCI_BUS_ID: &str = "00000000:01:00.0";` `crates/whisper-core/src/model_registry.rs:40`; `pub const PARAKEET_TEST_GPU_UUID: &str = "GPU-99e636f6-45ac-0068-a378-7d7a8bca645e";` `crates/whisper-core/src/model_registry.rs:41`; `pub const PARAKEET_TEST_GPU_PNP_INSTANCE_ID: &str =` `crates/whisper-core/src/model_registry.rs:42`; `pub const PARAKEET_PERSONAL_GPU_PCI_BUS_ID: &str = "00000000:01:00.0";` `crates/whisper-core/src/model_registry.rs:44`; `pub const PARAKEET_PERSONAL_GPU_UUID: &str = "GPU-99e636f6-45ac-0068-a378-7d7a8bca645e";` `crates/whisper-core/src/model_registry.rs:45`; `pub const PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID: &str =` `crates/whisper-core/src/model_registry.rs:46`; `pub const PARAKEET_PERSONAL_CPU_RUNTIME_SHA256: &str =` `crates/whisper-core/src/model_registry.rs:50`; `pub const PARAKEET_PERSONAL_CPU_WORKER_SHA256: &str =` `crates/whisper-core/src/model_registry.rs:52`; `pub const PARAKEET_PERSONAL_GPU_RUNTIME_SHA256: &str =` `crates/whisper-core/src/model_registry.rs:54`; `pub const PARAKEET_PERSONAL_GPU_WORKER_SHA256: &str =` `crates/whisper-core/src/model_registry.rs:56`; `pub const MODELS: [ModelRecord; 2] = [` `crates/whisper-core/src/model_registry.rs:59`; `pub const TURBO_LANGUAGE_INPUTS: [(&str, &str); 100] = [` `crates/whisper-core/src/model_registry.rs:85`; `pub fn model(id: ModelId) -> &'static ModelRecord {` `crates/whisper-core/src/model_registry.rs:188`; `pub fn engine_snapshot(` `crates/whisper-core/src/model_registry.rs:195`; `pub fn configuration_key(record: &ModelRecord, backend: Backend) -> ConfigurationKey {` `crates/whisper-core/src/model_registry.rs:267`; `pub fn personal_configuration_key(record: &ModelRecord, backend: Backend) -> ConfigurationKey {` `crates/whisper-core/src/model_registry.rs:314`.

Dépend de, via `use crate::` : hors tests : `crate::engine_capabilities::Capability` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities::ConfigurationCapabilities` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities::EngineSnapshot` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities::LanguageCapability` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities::LanguageCode` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities::Qualification` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities::Support` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::engine_capabilities` → `crate::engine_capabilities` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_catalog::Backend` → `crate::model_catalog` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_catalog::ConfigurationKey` → `crate::model_catalog` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_catalog::ModelAvailability` → `crate::model_catalog` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_catalog::ModelDefinition` → `crate::model_catalog` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_catalog::ModelId` → `crate::model_catalog` `crates/whisper-core/src/model_registry.rs:3`<br>hors tests : `crate::model_catalog` → `crate::model_catalog` `crates/whisper-core/src/model_registry.rs:3`.

Utilisé par, via ces imports : tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::engine_capabilities` `crates/whisper-core/src/engine_capabilities.rs:396`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_selection_app` `crates/whisper-core/src/model_selection_app.rs:133`.

### crate::model_selection_app — `crates/whisper-core/src/model_selection_app.rs`

Valide les demandes import/live contre catalogue et capacités. `crates/whisper-core/src/model_selection_app.rs:1-313`

Mesure : 313 production / 0 tests / 313 physiques. `crates/whisper-core/src/model_selection_app.rs:1-313`

Types : `ComputePreference` (enum, hors tests) `crates/whisper-core/src/model_selection_app.rs:11-15`; `InferencePath` (enum, hors tests) `crates/whisper-core/src/model_selection_app.rs:18-21`; `SelectionIntent` (struct, hors tests) `crates/whisper-core/src/model_selection_app.rs:24-33`; `AdmittedSelection` (struct, hors tests) `crates/whisper-core/src/model_selection_app.rs:36-41`; `SelectionRefusal` (enum, hors tests) `crates/whisper-core/src/model_selection_app.rs:44-47`; `ModelSelectionApp` (struct, hors tests) `crates/whisper-core/src/model_selection_app.rs:49-51`; `ModelCommandGuard` (struct, hors tests) `crates/whisper-core/src/model_selection_app.rs:55-59`.

Fonctions avec corps : `new` `crates/whisper-core/src/model_selection_app.rs:62-74`; `new_with_model_paths` `crates/whisper-core/src/model_selection_app.rs:76-86`; `admit_import` `crates/whisper-core/src/model_selection_app.rs:88-108`; `admit_live` `crates/whisper-core/src/model_selection_app.rs:110-122`; `admit_fields` `crates/whisper-core/src/model_selection_app.rs:124-207`; `decode_sha256` `crates/whisper-core/src/model_selection_app.rs:210-219`; `new` `crates/whisper-core/src/model_selection_app.rs:222-224`; `projection` `crates/whisper-core/src/model_selection_app.rs:226-228`; `admit` `crates/whisper-core/src/model_selection_app.rs:231-274`; `is_known_historical_path` `crates/whisper-core/src/model_selection_app.rs:278-280`; `resolve_legacy_model_path` `crates/whisper-core/src/model_selection_app.rs:282-289`; `normalize_windows_path` `crates/whisper-core/src/model_selection_app.rs:291-293`; `normalize_language` `crates/whisper-core/src/model_selection_app.rs:295-313`.

Déclarations `pub` (visibilité sous conditions) : `pub enum ComputePreference {` `crates/whisper-core/src/model_selection_app.rs:11`; `pub enum InferencePath {` `crates/whisper-core/src/model_selection_app.rs:18`; `pub struct SelectionIntent {` `crates/whisper-core/src/model_selection_app.rs:24`; `pub struct AdmittedSelection {` `crates/whisper-core/src/model_selection_app.rs:36`; `pub enum SelectionRefusal {` `crates/whisper-core/src/model_selection_app.rs:44`; `pub struct ModelSelectionApp {` `crates/whisper-core/src/model_selection_app.rs:49`; `pub struct ModelCommandGuard {` `crates/whisper-core/src/model_selection_app.rs:55`; `pub fn new(` `crates/whisper-core/src/model_selection_app.rs:62`; `pub fn new_with_model_paths(` `crates/whisper-core/src/model_selection_app.rs:76`; `pub fn admit_import(` `crates/whisper-core/src/model_selection_app.rs:88`; `pub fn admit_live(` `crates/whisper-core/src/model_selection_app.rs:110`; `pub fn new(snapshot: EngineSnapshot) -> Self {` `crates/whisper-core/src/model_selection_app.rs:222`; `pub fn projection(&self, current_generation: u64) -> Result<EngineProjection, Refusal> {` `crates/whisper-core/src/model_selection_app.rs:226`; `pub fn admit(` `crates/whisper-core/src/model_selection_app.rs:231`; `pub fn is_known_historical_path(path: &str, known_path: &str) -> bool {` `crates/whisper-core/src/model_selection_app.rs:278`; `pub fn resolve_legacy_model_path(` `crates/whisper-core/src/model_selection_app.rs:282`.

Dépend de, via `use crate::` : hors tests : `crate::engine_capabilities::Admission` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::EngineProjection` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::EngineSnapshot` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::InferenceRequest` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::LanguageChoice` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::LanguageCode` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::Refusal` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::project_with_existing_recognized` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities::validate_with_existing_recognized` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::engine_capabilities` → `crate::engine_capabilities` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_catalog::Backend` → `crate::model_catalog` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_catalog::ModelId` → `crate::model_catalog` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_catalog` → `crate::model_catalog` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::model_registry::TURBO_LANGUAGE_INPUTS` → `crate::model_registry` `crates/whisper-core/src/model_selection_app.rs:1`<br>hors tests : `crate::domain::ComputeChoice` → `crate::domain` `crates/whisper-core/src/model_selection_app.rs:133`<br>hors tests : `crate::model_registry::model` → `crate::model_registry` `crates/whisper-core/src/model_selection_app.rs:133`.

Utilisé par, via ces imports : tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`<br>tests : `crate::application::runtime::tests` `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:915`.

### crate::parakeet_ipc — `crates/whisper-core/src/parakeet_ipc.rs`

Définit commandes, événements, identité et preuves GPU du protocole Parakeet. `crates/whisper-core/src/parakeet_ipc.rs:1-130`

Mesure : 130 production / 0 tests / 130 physiques. `crates/whisper-core/src/parakeet_ipc.rs:1-130`

Types : `WorkerIdentity` (struct, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:9-18`; `Envelope` (struct, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:21-24`; `ExpectedGpu` (struct, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:29-32`; `GpuEvidence` (struct, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:37-43`; `Command` (enum, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:56-69`; `Event` (enum, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:73-116`; `Word` (struct, hors tests) `crates/whisper-core/src/parakeet_ipc.rs:126-130`.

Fonctions avec corps : `new` `crates/whisper-core/src/parakeet_ipc.rs:46-51`.

Déclarations `pub` (visibilité sous conditions) : `pub const PARAKEET_IPC_VERSION: u16 = 1;` `crates/whisper-core/src/parakeet_ipc.rs:3`; `pub const PARAKEET_MAX_FRAME_BYTES: usize = 1_048_576;` `crates/whisper-core/src/parakeet_ipc.rs:4`; `pub const PARAKEET_MAX_AUDIO_SAMPLES: usize = 28_800_000;` `crates/whisper-core/src/parakeet_ipc.rs:5`; `pub const PARAKEET_PROFILE_IMPORT_SINGLE_PASS: &str = "import-single-pass";` `crates/whisper-core/src/parakeet_ipc.rs:6`; `pub struct WorkerIdentity {` `crates/whisper-core/src/parakeet_ipc.rs:9`; `pub struct Envelope<T> {` `crates/whisper-core/src/parakeet_ipc.rs:21`; `pub struct ExpectedGpu {` `crates/whisper-core/src/parakeet_ipc.rs:29`; `pub struct GpuEvidence {` `crates/whisper-core/src/parakeet_ipc.rs:37`; `pub fn new(message: T) -> Self {` `crates/whisper-core/src/parakeet_ipc.rs:46`; `pub enum Command {` `crates/whisper-core/src/parakeet_ipc.rs:56`; `pub enum Event {` `crates/whisper-core/src/parakeet_ipc.rs:73`; `pub struct Word {` `crates/whisper-core/src/parakeet_ipc.rs:126`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/src/parakeet_ipc.rs:1-130`.

Utilisé par, via ces imports : hors tests : `crate::parakeet_windows` `crates/whisper-core/src/parakeet_windows.rs:3`.

### crate::parakeet_scheduler — `crates/whisper-core/src/parakeet_scheduler.rs`

Ordonne les appels preview/final et suit admission, exécution et recyclage. `crates/whisper-core/src/parakeet_scheduler.rs:1-672`

Mesure : 424 production / 248 tests / 672 physiques. `crates/whisper-core/src/parakeet_scheduler.rs:1-672`

Types : `SchedulerSession` (struct, hors tests) `crates/whisper-core/src/parakeet_scheduler.rs:10-13`; `ScheduledWork` (enum, hors tests) `crates/whisper-core/src/parakeet_scheduler.rs:16-19`; `ScheduledCall` (struct, hors tests) `crates/whisper-core/src/parakeet_scheduler.rs:22-28`; `SchedulerError` (enum, hors tests) `crates/whisper-core/src/parakeet_scheduler.rs:31-42`; `SessionState` (struct, hors tests) `crates/whisper-core/src/parakeet_scheduler.rs:45-52`; `ParakeetScheduler` (struct, hors tests) `crates/whisper-core/src/parakeet_scheduler.rs:57-67`.

Fonctions avec corps : `admit` `crates/whisper-core/src/parakeet_scheduler.rs:70-107`; `queue_preview` `crates/whisper-core/src/parakeet_scheduler.rs:109-123`; `queue_final` `crates/whisper-core/src/parakeet_scheduler.rs:125-144`; `clear_preview` `crates/whisper-core/src/parakeet_scheduler.rs:146-153`; `next_call` `crates/whisper-core/src/parakeet_scheduler.rs:155-190`; `begin_call` `crates/whisper-core/src/parakeet_scheduler.rs:195-225`; `cancel_reservation` `crates/whisper-core/src/parakeet_scheduler.rs:227-248`; `complete` `crates/whisper-core/src/parakeet_scheduler.rs:250-283`; `fail_call` `crates/whisper-core/src/parakeet_scheduler.rs:288-313`; `mark_terminal` `crates/whisper-core/src/parakeet_scheduler.rs:315-326`; `abort_session` `crates/whisper-core/src/parakeet_scheduler.rs:328-338`; `request_recycle` `crates/whisper-core/src/parakeet_scheduler.rs:342-345`; `confirm_child_exited` `crates/whisper-core/src/parakeet_scheduler.rs:347-372`; `active_sessions` `crates/whisper-core/src/parakeet_scheduler.rs:374-376`; `in_flight` `crates/whisper-core/src/parakeet_scheduler.rs:378-380`; `select_preview` `crates/whisper-core/src/parakeet_scheduler.rs:382-394`; `select_final` `crates/whisper-core/src/parakeet_scheduler.rs:396-413`; `recycle_if_idle` `crates/whisper-core/src/parakeet_scheduler.rs:415-422`; `session` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:429-434`; `preview_is_replaceable_and_final_work_has_p1_priority` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:437-456`; `preview_arriving_during_an_inflight_preview_survives_its_completion` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:459-477`; `final_sessions_alternate_and_old_incarnations_do_not_alias_requests` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:480-507`; `recycling_waits_for_the_real_inflight_exit_and_preserves_other_session_attempts` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:510-543`; `many_windows_reuse_two_session_records_without_growing_the_table` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:546-562`; `a_third_nonterminal_session_is_refused` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:565-571`; `preview_starvation_witness_fails_the_p1_contract` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:574-614`; `unrecycled_table_growth_witness_fails_the_two_session_bound` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:617-644`; `reserved_final_turn_covers_preparation_and_ready_wait` [tests] `crates/whisper-core/src/parakeet_scheduler.rs:647-671`.

Déclarations `pub` (visibilité sous conditions) : `pub struct SchedulerSession {` `crates/whisper-core/src/parakeet_scheduler.rs:10`; `pub enum ScheduledWork {` `crates/whisper-core/src/parakeet_scheduler.rs:16`; `pub struct ScheduledCall {` `crates/whisper-core/src/parakeet_scheduler.rs:22`; `pub enum SchedulerError {` `crates/whisper-core/src/parakeet_scheduler.rs:31`; `pub struct ParakeetScheduler {` `crates/whisper-core/src/parakeet_scheduler.rs:57`; `pub fn admit(&mut self, session: SchedulerSession) -> Result<u64, SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:70`; `pub fn queue_preview(` `crates/whisper-core/src/parakeet_scheduler.rs:109`; `pub fn queue_final(` `crates/whisper-core/src/parakeet_scheduler.rs:125`; `pub fn clear_preview(&mut self, session: SchedulerSession) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:146`; `pub fn next_call(&mut self) -> Result<Option<ScheduledCall>, SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:155`; `pub fn begin_call(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:195`; `pub fn cancel_reservation(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:227`; `pub fn complete(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:250`; `pub fn fail_call(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:288`; `pub fn mark_terminal(&mut self, session: SchedulerSession) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:315`; `pub fn abort_session(&mut self, session: SchedulerSession) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:328`; `pub fn request_recycle(&mut self) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:342`; `pub fn confirm_child_exited(&mut self) -> Result<(), SchedulerError> {` `crates/whisper-core/src/parakeet_scheduler.rs:347`; `pub fn active_sessions(&self) -> usize {` `crates/whisper-core/src/parakeet_scheduler.rs:374`; `pub fn in_flight(&self) -> Option<ScheduledCall> {` `crates/whisper-core/src/parakeet_scheduler.rs:378`.

Dépend de, via `use crate::` : hors tests : `crate::Generation` → [Non vérifié] symbole/réexport non résolu `crates/whisper-core/src/parakeet_scheduler.rs:5`<br>hors tests : `crate::JobId` → [Non vérifié] symbole/réexport non résolu `crates/whisper-core/src/parakeet_scheduler.rs:5`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `session` `crates/whisper-core/src/parakeet_scheduler.rs:429-434`, `preview_is_replaceable_and_final_work_has_p1_priority` `crates/whisper-core/src/parakeet_scheduler.rs:437-456`, `preview_arriving_during_an_inflight_preview_survives_its_completion` `crates/whisper-core/src/parakeet_scheduler.rs:459-477`, `final_sessions_alternate_and_old_incarnations_do_not_alias_requests` `crates/whisper-core/src/parakeet_scheduler.rs:480-507`. `crates/whisper-core/src/parakeet_scheduler.rs:425-672`

Mesure locale : 0 production / 248 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/parakeet_scheduler.rs:425-672`

Types locaux : aucun relevé dans `crates/whisper-core/src/parakeet_scheduler.rs:425-672`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/parakeet_scheduler.rs:425-672`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_session — `crates/whisper-core/src/parakeet_session.rs`

Définit stratégies et transitions des frontières/fenêtres de session. `crates/whisper-core/src/parakeet_session.rs:1-433`

Mesure : 309 production / 124 tests / 433 physiques. `crates/whisper-core/src/parakeet_session.rs:1-433`

Types : `SessionIdentity` (struct, hors tests) `crates/whisper-core/src/parakeet_session.rs:15-19`; `TranscriptionStrategy` (enum, hors tests) `crates/whisper-core/src/parakeet_session.rs:23-26`; `AudioFrontiers` (struct, hors tests) `crates/whisper-core/src/parakeet_session.rs:38-43`; `FrontierError` (enum, hors tests) `crates/whisper-core/src/parakeet_session.rs:46-49`; `AudioFrontier` (enum, hors tests) `crates/whisper-core/src/parakeet_session.rs:84-89`; `DocumentRevision` (struct, hors tests) `crates/whisper-core/src/parakeet_session.rs:92-95`; `AudioWindow` (struct, hors tests) `crates/whisper-core/src/parakeet_session.rs:117-123`; `WindowCursorError` (enum, hors tests) `crates/whisper-core/src/parakeet_session.rs:126-130`; `WindowCursor` (struct, hors tests) `crates/whisper-core/src/parakeet_session.rs:135-138`; `WindowCursorCheckpoint` (struct, hors tests) `crates/whisper-core/src/parakeet_session.rs:141-144`.

Fonctions avec corps : `stable_id` `crates/whisper-core/src/parakeet_session.rs:29-34`; `advance` `crates/whisper-core/src/parakeet_session.rs:54-80`; `revise` `crates/whisper-core/src/parakeet_session.rs:98-104`; `finalize` `crates/whisper-core/src/parakeet_session.rs:105-113`; `default` `crates/whisper-core/src/parakeet_session.rs:147-152`; `from_checkpoint` `crates/whisper-core/src/parakeet_session.rs:156-167`; `checkpoint` `crates/whisper-core/src/parakeet_session.rs:169-174`; `next_start` `crates/whisper-core/src/parakeet_session.rs:176-178`; `next_window` `crates/whisper-core/src/parakeet_session.rs:180-245`; `emit` `crates/whisper-core/src/parakeet_session.rs:247-268`; `find_silences` `crates/whisper-core/src/parakeet_session.rs:271-298`; `add_silence_center` `crates/whisper-core/src/parakeet_session.rs:300-308`; `frontiers_are_distinct_monotonic_and_ordered` [tests] `crates/whisper-core/src/parakeet_session.rs:315-338`; `continuous_speech_falls_back_to_120_seconds_and_cursor_is_contiguous` [tests] `crates/whisper-core/src/parakeet_session.rs:341-367`; `final_revision_is_stable_and_cannot_be_revised` [tests] `crates/whisper-core/src/parakeet_session.rs:370-379`; `window_cursor_checkpoint_restores_exact_next_window` [tests] `crates/whisper-core/src/parakeet_session.rs:382-405`; `window_cursor_rejects_inconsistent_checkpoint_indices` [tests] `crates/whisper-core/src/parakeet_session.rs:408-432`.

Déclarations `pub` (visibilité sous conditions) : `pub struct SessionIdentity {` `crates/whisper-core/src/parakeet_session.rs:15`; `pub enum TranscriptionStrategy {` `crates/whisper-core/src/parakeet_session.rs:23`; `pub const fn stable_id(self) -> &'static str {` `crates/whisper-core/src/parakeet_session.rs:29`; `pub struct AudioFrontiers {` `crates/whisper-core/src/parakeet_session.rs:38`; `pub enum FrontierError {` `crates/whisper-core/src/parakeet_session.rs:46`; `pub fn advance(&mut self, frontier: AudioFrontier, sample: u64) -> Result<(), FrontierError> {` `crates/whisper-core/src/parakeet_session.rs:54`; `pub enum AudioFrontier {` `crates/whisper-core/src/parakeet_session.rs:84`; `pub struct DocumentRevision {` `crates/whisper-core/src/parakeet_session.rs:92`; `pub fn revise(&mut self) -> Option<u64> {` `crates/whisper-core/src/parakeet_session.rs:98`; `pub fn finalize(&mut self) -> Option<u64> {` `crates/whisper-core/src/parakeet_session.rs:105`; `pub struct AudioWindow {` `crates/whisper-core/src/parakeet_session.rs:117`; `pub enum WindowCursorError {` `crates/whisper-core/src/parakeet_session.rs:126`; `pub struct WindowCursor {` `crates/whisper-core/src/parakeet_session.rs:135`; `pub struct WindowCursorCheckpoint {` `crates/whisper-core/src/parakeet_session.rs:141`; `pub fn from_checkpoint(checkpoint: WindowCursorCheckpoint) -> Result<Self, WindowCursorError> {` `crates/whisper-core/src/parakeet_session.rs:156`; `pub fn checkpoint(&self) -> WindowCursorCheckpoint {` `crates/whisper-core/src/parakeet_session.rs:169`; `pub fn next_start(&self) -> u64 {` `crates/whisper-core/src/parakeet_session.rs:176`; `pub fn next_window(` `crates/whisper-core/src/parakeet_session.rs:180`.

Dépend de, via `use crate::` : hors tests : `crate::domain::Generation` → `crate::domain` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::domain::JobId` → `crate::domain` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_windows::FRAME_SAMPLES` → `crate::parakeet_windows` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_windows::MARGIN_SAMPLES` → `crate::parakeet_windows` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_windows::WINDOW_MAX_SAMPLES` → `crate::parakeet_windows` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_windows::WINDOW_MIN_SAMPLES` → `crate::parakeet_windows` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_windows::WINDOW_TARGET_SAMPLES` → `crate::parakeet_windows` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_windows` → `crate::parakeet_windows` `crates/whisper-core/src/parakeet_session.rs:3`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `frontiers_are_distinct_monotonic_and_ordered` `crates/whisper-core/src/parakeet_session.rs:315-338`, `continuous_speech_falls_back_to_120_seconds_and_cursor_is_contiguous` `crates/whisper-core/src/parakeet_session.rs:341-367`, `final_revision_is_stable_and_cannot_be_revised` `crates/whisper-core/src/parakeet_session.rs:370-379`, `window_cursor_checkpoint_restores_exact_next_window` `crates/whisper-core/src/parakeet_session.rs:382-405`. `crates/whisper-core/src/parakeet_session.rs:310-433`

Mesure locale : 0 production / 124 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/parakeet_session.rs:310-433`

Types locaux : aucun relevé dans `crates/whisper-core/src/parakeet_session.rs:310-433`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/parakeet_session.rs:310-433`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_windows — `crates/whisper-core/src/parakeet_windows.rs`

Planifie les fenêtres et convertit les temps des mots en échantillons absolus. `crates/whisper-core/src/parakeet_windows.rs:1-333`

Mesure : 273 production / 60 tests / 333 physiques. `crates/whisper-core/src/parakeet_windows.rs:1-333`

Types : `Silence` (struct, hors tests) `crates/whisper-core/src/parakeet_windows.rs:19-23`; `Window` (struct, hors tests) `crates/whisper-core/src/parakeet_windows.rs:26-33`; `AbsoluteWordSamples` (struct, hors tests) `crates/whisper-core/src/parakeet_windows.rs:36-39`; `TimingError` (enum, hors tests) `crates/whisper-core/src/parakeet_windows.rs:42-47`; `WindowError` (enum, hors tests) `crates/whisper-core/src/parakeet_windows.rs:102-106`.

Fonctions avec corps : `fmt` `crates/whisper-core/src/parakeet_windows.rs:50-52`; `absolute_word_samples` `crates/whisper-core/src/parakeet_windows.rs:59-99`; `fmt` `crates/whisper-core/src/parakeet_windows.rs:109-118`; `admissible_silences` `crates/whisper-core/src/parakeet_windows.rs:122-143`; `push_silence` `crates/whisper-core/src/parakeet_windows.rs:145-156`; `plan_windows` `crates/whisper-core/src/parakeet_windows.rs:159-219`; `validate_reference_duration` `crates/whisper-core/src/parakeet_windows.rs:223-231`; `make_window` `crates/whisper-core/src/parakeet_windows.rs:233-260`; `word_belongs_to_central_range` `crates/whisper-core/src/parakeet_windows.rs:263-272`; `pcm_with_level` [tests] `crates/whisper-core/src/parakeet_windows.rs:278-280`; `rms_64_frame_threshold_and_400ms_run_are_exact` [tests] `crates/whisper-core/src/parakeet_windows.rs:283-290`; `silence_is_centered_and_earliest_wins_equal_distance` [tests] `crates/whisper-core/src/parakeet_windows.rs:293-305`; `word_center_has_one_owner_at_joint_boundary` [tests] `crates/whisper-core/src/parakeet_windows.rs:308-322`; `reference_limit_does_not_cap_product_window_planning` [tests] `crates/whisper-core/src/parakeet_windows.rs:325-332`.

Déclarations `pub` (visibilité sous conditions) : `pub const SAMPLE_RATE: u64 = 16_000;` `crates/whisper-core/src/parakeet_windows.rs:5`; `pub const FRAME_SAMPLES: usize = 320;` `crates/whisper-core/src/parakeet_windows.rs:6`; `pub const WINDOW_MIN_SAMPLES: u64 = 60 * SAMPLE_RATE;` `crates/whisper-core/src/parakeet_windows.rs:7`; `pub const WINDOW_TARGET_SAMPLES: u64 = 90 * SAMPLE_RATE;` `crates/whisper-core/src/parakeet_windows.rs:8`; `pub const WINDOW_MAX_SAMPLES: u64 = 120 * SAMPLE_RATE;` `crates/whisper-core/src/parakeet_windows.rs:9`; `pub const MARGIN_SAMPLES: u64 = 200 * SAMPLE_RATE / 1_000;` `crates/whisper-core/src/parakeet_windows.rs:10`; `pub const SILENCE_MIN_FRAMES: usize = 20;` `crates/whisper-core/src/parakeet_windows.rs:11`; `pub const MAX_NATIVE_SAMPLES: u64 = 1_800 * SAMPLE_RATE;` `crates/whisper-core/src/parakeet_windows.rs:12`; `pub const MAX_REFERENCE_SAMPLES: u64 = 3_600 * SAMPLE_RATE;` `crates/whisper-core/src/parakeet_windows.rs:13`; `pub const PROFILE_IMPORT_VAD: &str = "import-vad";` `crates/whisper-core/src/parakeet_windows.rs:14`; `pub const PROFILE_QUALIFICATION_REFERENCE: &str = "qualification-reference";` `crates/whisper-core/src/parakeet_windows.rs:15`; `pub struct Silence {` `crates/whisper-core/src/parakeet_windows.rs:19`; `pub struct Window {` `crates/whisper-core/src/parakeet_windows.rs:26`; `pub struct AbsoluteWordSamples {` `crates/whisper-core/src/parakeet_windows.rs:36`; `pub enum TimingError {` `crates/whisper-core/src/parakeet_windows.rs:42`; `pub fn absolute_word_samples(` `crates/whisper-core/src/parakeet_windows.rs:59`; `pub enum WindowError {` `crates/whisper-core/src/parakeet_windows.rs:102`; `pub fn admissible_silences(samples: &[i16]) -> Vec<Silence> {` `crates/whisper-core/src/parakeet_windows.rs:122`; `pub fn plan_windows(samples: &[i16]) -> Result<Vec<Window>, WindowError> {` `crates/whisper-core/src/parakeet_windows.rs:159`; `pub fn validate_reference_duration(total_samples: u64) -> Result<(), WindowError> {` `crates/whisper-core/src/parakeet_windows.rs:223`; `pub fn word_belongs_to_central_range(` `crates/whisper-core/src/parakeet_windows.rs:263`.

Dépend de, via `use crate::` : hors tests : `crate::parakeet_ipc::Word` → `crate::parakeet_ipc` `crates/whisper-core/src/parakeet_windows.rs:3`.

Utilisé par, via ces imports : hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`<br>hors tests : `crate::parakeet_session` `crates/whisper-core/src/parakeet_session.rs:3`.

#### Inline `tests`

Tests/auxiliaires : `pcm_with_level` `crates/whisper-core/src/parakeet_windows.rs:278-280`, `rms_64_frame_threshold_and_400ms_run_are_exact` `crates/whisper-core/src/parakeet_windows.rs:283-290`, `silence_is_centered_and_earliest_wins_equal_distance` `crates/whisper-core/src/parakeet_windows.rs:293-305`, `word_center_has_one_owner_at_joint_boundary` `crates/whisper-core/src/parakeet_windows.rs:308-322`. `crates/whisper-core/src/parakeet_windows.rs:274-333`

Mesure locale : 0 production / 60 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/parakeet_windows.rs:274-333`

Types locaux : aucun relevé dans `crates/whisper-core/src/parakeet_windows.rs:274-333`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-core/src/parakeet_windows.rs:274-333`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::path_settings — `crates/whisper-core/src/path_settings.rs`

Restaure et mémorise chemins et préférences au travers de ports. `crates/whisper-core/src/path_settings.rs:1-851`

Mesure : 616 production / 235 tests / 851 physiques. `crates/whisper-core/src/path_settings.rs:1-851`

Types : `PathField` (enum, hors tests) `crates/whisper-core/src/path_settings.rs:4-8`; `StoredPaths` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:11-22`; `GainDbEntry` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:40`; `GainDbEntryVisitor` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:51`; `Value` (type, hors tests) `crates/whisper-core/src/path_settings.rs:54`; `SaveStatus` (enum, hors tests) `crates/whisper-core/src/path_settings.rs:106-111`; `SaveAcknowledgement` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:114-117`; `StartupComputePreference` (enum, hors tests) `crates/whisper-core/src/path_settings.rs:120-123`; `LegacyModelPath` (enum, hors tests) `crates/whisper-core/src/path_settings.rs:126-130`; `ModelPreferenceSnapshot` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:133-138`; `ModelPreferenceAcknowledgement` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:141-144`; `ModelPreferencePort` (trait, hors tests) `crates/whisper-core/src/path_settings.rs:146-154`; `ModelPreferenceSaveStatus` (enum, hors tests) `crates/whisper-core/src/path_settings.rs:157-160`; `ModelPreferencesApp` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:162-169`; `PathPort` (trait, hors tests) `crates/whisper-core/src/path_settings.rs:249-256`; `PathDefaults` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:258-262`; `NullPathPort` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:264`; `PathSettingsApp` (struct, hors tests) `crates/whisper-core/src/path_settings.rs:287-297`; `MemoryPort` (struct, tests) `crates/whisper-core/src/path_settings.rs:627`; `AckPort` (struct, tests) `crates/whisper-core/src/path_settings.rs:629-633`.

Fonctions avec corps : `deserialize_microphone_gains` `crates/whisper-core/src/path_settings.rs:24-38`; `deserialize` `crates/whisper-core/src/path_settings.rs:43-48`; `expecting` `crates/whisper-core/src/path_settings.rs:56-58`; `visit_i64` `crates/whisper-core/src/path_settings.rs:60-62`; `visit_u64` `crates/whisper-core/src/path_settings.rs:63-65`; `visit_bool` `crates/whisper-core/src/path_settings.rs:66-68`; `visit_f64` `crates/whisper-core/src/path_settings.rs:69-71`; `visit_str` `crates/whisper-core/src/path_settings.rs:72-74`; `visit_string` `crates/whisper-core/src/path_settings.rs:75-77`; `visit_none` `crates/whisper-core/src/path_settings.rs:78-80`; `visit_unit` `crates/whisper-core/src/path_settings.rs:81-83`; `visit_seq` `crates/whisper-core/src/path_settings.rs:85-91`; `visit_map` `crates/whisper-core/src/path_settings.rs:93-102`; `new` `crates/whisper-core/src/path_settings.rs:172-195`; `choose_model` `crates/whisper-core/src/path_settings.rs:197-200`; `choose_startup_compute` `crates/whisper-core/src/path_settings.rs:202-205`; `save_status` `crates/whisper-core/src/path_settings.rs:207-209`; `legacy_path_notice` `crates/whisper-core/src/path_settings.rs:211-221`; `persist` `crates/whisper-core/src/path_settings.rs:223-246`; `load` `crates/whisper-core/src/path_settings.rs:267-269`; `save` `crates/whisper-core/src/path_settings.rs:270-272`; `pick` `crates/whisper-core/src/path_settings.rs:273-275`; `exists` `crates/whisper-core/src/path_settings.rs:276-278`; `has_pending_save` `crates/whisper-core/src/path_settings.rs:279-281`; `poll_save_acknowledgement` `crates/whisper-core/src/path_settings.rs:282-284`; `with_defaults` `crates/whisper-core/src/path_settings.rs:300-302`; `new` `crates/whisper-core/src/path_settings.rs:304-339`; `restore` `crates/whisper-core/src/path_settings.rs:341-403`; `browse` `crates/whisper-core/src/path_settings.rs:405-448`; `commit_manual` `crates/whisper-core/src/path_settings.rs:450-492`; `commit_all` `crates/whisper-core/src/path_settings.rs:494-498`; `microphone_preference` `crates/whisper-core/src/path_settings.rs:500-505`; `set_microphone_preference` `crates/whisper-core/src/path_settings.rs:507-513`; `microphone_gain` `crates/whisper-core/src/path_settings.rs:515-525`; `set_microphone_gain` `crates/whisper-core/src/path_settings.rs:527-541`; `gpu_preference` `crates/whisper-core/src/path_settings.rs:543-545`; `set_gpu_preference` `crates/whisper-core/src/path_settings.rs:547-550`; `save_status` `crates/whisper-core/src/path_settings.rs:552-554`; `poll_notice` `crates/whisper-core/src/path_settings.rs:556-581`; `has_pending_save` `crates/whisper-core/src/path_settings.rs:583-585`; `take_startup_notice` `crates/whisper-core/src/path_settings.rs:587-589`; `save` `crates/whisper-core/src/path_settings.rs:591-614`; `load` [tests] `crates/whisper-core/src/path_settings.rs:636-638`; `save` [tests] `crates/whisper-core/src/path_settings.rs:640-646`; `pick` [tests] `crates/whisper-core/src/path_settings.rs:648-650`; `exists` [tests] `crates/whisper-core/src/path_settings.rs:652-654`; `has_pending_save` [tests] `crates/whisper-core/src/path_settings.rs:656-658`; `poll_save_acknowledgement` [tests] `crates/whisper-core/src/path_settings.rs:660-662`; `load` [tests] `crates/whisper-core/src/path_settings.rs:666-668`; `save` [tests] `crates/whisper-core/src/path_settings.rs:670-673`; `pick` [tests] `crates/whisper-core/src/path_settings.rs:675-677`; `exists` [tests] `crates/whisper-core/src/path_settings.rs:679-681`; `has_pending_save` [tests] `crates/whisper-core/src/path_settings.rs:683-685`; `poll_save_acknowledgement` [tests] `crates/whisper-core/src/path_settings.rs:687-689`; `microphone_preference_survives_path_save_and_reopen` [tests] `crates/whisper-core/src/path_settings.rs:693-709`; `microphone_gains_are_persisted_per_identity_and_neutral_is_absent` [tests] `crates/whisper-core/src/path_settings.rs:712-745`; `invalid_gain_is_neutralized_alone_and_reported` [tests] `crates/whisper-core/src/path_settings.rs:748-786`; `stale_save_ack_is_ignored_and_failed_latest_save_is_visible_without_losing_live_gain` [tests] `crates/whisper-core/src/path_settings.rs:789-850`.

Déclarations `pub` (visibilité sous conditions) : `pub enum PathField {` `crates/whisper-core/src/path_settings.rs:4`; `pub struct StoredPaths {` `crates/whisper-core/src/path_settings.rs:11`; `pub enum SaveStatus {` `crates/whisper-core/src/path_settings.rs:106`; `pub struct SaveAcknowledgement {` `crates/whisper-core/src/path_settings.rs:114`; `pub enum StartupComputePreference {` `crates/whisper-core/src/path_settings.rs:120`; `pub enum LegacyModelPath {` `crates/whisper-core/src/path_settings.rs:126`; `pub struct ModelPreferenceSnapshot {` `crates/whisper-core/src/path_settings.rs:133`; `pub struct ModelPreferenceAcknowledgement {` `crates/whisper-core/src/path_settings.rs:141`; `pub trait ModelPreferencePort {` `crates/whisper-core/src/path_settings.rs:146`; `pub enum ModelPreferenceSaveStatus {` `crates/whisper-core/src/path_settings.rs:157`; `pub struct ModelPreferencesApp {` `crates/whisper-core/src/path_settings.rs:162`; `pub fn new(port: Box<dyn ModelPreferencePort>) -> Result<Self, String> {` `crates/whisper-core/src/path_settings.rs:172`; `pub fn choose_model(&mut self, model: crate::model_catalog::ModelId) {` `crates/whisper-core/src/path_settings.rs:197`; `pub fn choose_startup_compute(&mut self, preference: StartupComputePreference) {` `crates/whisper-core/src/path_settings.rs:202`; `pub fn save_status(&self) -> Option<&ModelPreferenceSaveStatus> {` `crates/whisper-core/src/path_settings.rs:207`; `pub fn legacy_path_notice(&self) -> Option<String> {` `crates/whisper-core/src/path_settings.rs:211`; `pub trait PathPort {` `crates/whisper-core/src/path_settings.rs:249`; `pub struct PathDefaults {` `crates/whisper-core/src/path_settings.rs:258`; `pub struct NullPathPort;` `crates/whisper-core/src/path_settings.rs:264`; `pub struct PathSettingsApp {` `crates/whisper-core/src/path_settings.rs:287`; `pub fn with_defaults(defaults: PathDefaults) -> Self {` `crates/whisper-core/src/path_settings.rs:300`; `pub fn new(defaults: PathDefaults, port: Box<dyn PathPort>) -> Self {` `crates/whisper-core/src/path_settings.rs:304`; `pub fn browse(&mut self, field: PathField) {` `crates/whisper-core/src/path_settings.rs:405`; `pub fn commit_manual(&mut self, field: PathField) {` `crates/whisper-core/src/path_settings.rs:450`; `pub fn commit_all(&mut self) {` `crates/whisper-core/src/path_settings.rs:494`; `pub fn microphone_preference(&self) -> crate::microphone::MicrophoneChoice {` `crates/whisper-core/src/path_settings.rs:500`; `pub fn set_microphone_preference(&mut self, choice: &crate::microphone::MicrophoneChoice) {` `crates/whisper-core/src/path_settings.rs:507`; `pub fn microphone_gain(` `crates/whisper-core/src/path_settings.rs:515`; `pub fn set_microphone_gain(` `crates/whisper-core/src/path_settings.rs:527`; `pub fn gpu_preference(&self) -> Option<&str> {` `crates/whisper-core/src/path_settings.rs:543`; `pub fn set_gpu_preference(&mut self, identity: Option<String>) {` `crates/whisper-core/src/path_settings.rs:547`; `pub fn save_status(&self) -> &SaveStatus {` `crates/whisper-core/src/path_settings.rs:552`; `pub fn poll_notice(&mut self) -> Option<String> {` `crates/whisper-core/src/path_settings.rs:556`; `pub fn has_pending_save(&self) -> bool {` `crates/whisper-core/src/path_settings.rs:583`; `pub fn take_startup_notice(&mut self) -> Option<String> {` `crates/whisper-core/src/path_settings.rs:587`.

Dépend de, via `use crate::` : tests : `crate::MicrophoneChoice` → [Non vérifié] symbole/réexport non résolu `crates/whisper-core/src/path_settings.rs:620`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `microphone_tests`

Tests/auxiliaires : `load` `crates/whisper-core/src/path_settings.rs:636-638`, `save` `crates/whisper-core/src/path_settings.rs:640-646`, `pick` `crates/whisper-core/src/path_settings.rs:648-650`, `exists` `crates/whisper-core/src/path_settings.rs:652-654`. `crates/whisper-core/src/path_settings.rs:617-851`

Mesure locale : 0 production / 235 tests ; sous-ensemble non additionnable. `crates/whisper-core/src/path_settings.rs:617-851`

Types locaux : `MemoryPort` `crates/whisper-core/src/path_settings.rs:627`, `AckPort` `crates/whisper-core/src/path_settings.rs:629-633`.

Imports locaux : `crate::MicrophoneChoice` `crates/whisper-core/src/path_settings.rs:620`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::ports — `crates/whisper-core/src/ports.rs`

Définit les ports et données échangées par effets et événements. `crates/whisper-core/src/ports.rs:1-873`

Mesure : 873 production / 0 tests / 873 physiques. `crates/whisper-core/src/ports.rs:1-873`

Types : `PortError` (enum, hors tests) `crates/whisper-core/src/ports.rs:6-14`; `Clock` (trait, hors tests) `crates/whisper-core/src/ports.rs:16-18`; `JobRepository` (trait, hors tests) `crates/whisper-core/src/ports.rs:20-23`; `PcmBlock` (struct, hors tests) `crates/whisper-core/src/ports.rs:26-31`; `SourceIdentity` (struct, hors tests) `crates/whisper-core/src/ports.rs:34-38`; `DecodeRequest` (struct, hors tests) `crates/whisper-core/src/ports.rs:41-47`; `DecodedPcmBlock` (struct, hors tests) `crates/whisper-core/src/ports.rs:50-55`; `DecoderPort` (trait, hors tests) `crates/whisper-core/src/ports.rs:57-59`; `WorkerRequest` (struct, hors tests) `crates/whisper-core/src/ports.rs:62-68`; `WorkerSegment` (struct, hors tests) `crates/whisper-core/src/ports.rs:71-78`; `WorkerPort` (trait, hors tests) `crates/whisper-core/src/ports.rs:80-84`; `ParakeetDocumentEntry` (struct, hors tests) `crates/whisper-core/src/ports.rs:88-91`; `ParakeetDocumentPage` (struct, hors tests) `crates/whisper-core/src/ports.rs:96-100`; `ParakeetPreview` (struct, hors tests) `crates/whisper-core/src/ports.rs:104-107`; `UnifiedSessionPhase` (enum, hors tests) `crates/whisper-core/src/ports.rs:110-117`; `UnifiedAudioPart` (struct, hors tests) `crates/whisper-core/src/ports.rs:120-125`; `UnifiedAudioPartPage` (struct, hors tests) `crates/whisper-core/src/ports.rs:128-133`; `UnifiedParakeetSessionView` (struct, hors tests) `crates/whisper-core/src/ports.rs:136-152`; `UnifiedParakeetSelectionView` (struct, hors tests) `crates/whisper-core/src/ports.rs:155-163`; `UnifiedPublicationFiles` (struct, hors tests) `crates/whisper-core/src/ports.rs:166-170`; `ParakeetWindowCoverage` (struct, hors tests) `crates/whisper-core/src/ports.rs:173-177`; `ParakeetDocumentPort` (trait, hors tests) `crates/whisper-core/src/ports.rs:179-213`; `ImportRequest` (struct, hors tests) `crates/whisper-core/src/ports.rs:216-228`; `DuplicateImportChoice` (enum, hors tests) `crates/whisper-core/src/ports.rs:235-244`; `AudioIdentity` (struct, hors tests) `crates/whisper-core/src/ports.rs:250-253`; `AudioFormat` (enum, hors tests) `crates/whisper-core/src/ports.rs:256-260`; `TreatmentIdentity` (struct, hors tests) `crates/whisper-core/src/ports.rs:264-268`; `OperationIdentity` (struct, hors tests) `crates/whisper-core/src/ports.rs:272-275`; `ReprocessingTarget` (enum, hors tests) `crates/whisper-core/src/ports.rs:280-285`; `ReprocessingStage` (enum, hors tests) `crates/whisper-core/src/ports.rs:288-296`; `ReprocessingProgress` (struct, hors tests) `crates/whisper-core/src/ports.rs:299-304`; `PublishedImport` (struct, hors tests) `crates/whisper-core/src/ports.rs:307-318`; `DuplicateImportPrompt` (struct, hors tests) `crates/whisper-core/src/ports.rs:321-341`; `LiveRequest` (struct, hors tests) `crates/whisper-core/src/ports.rs:344-361`; `AutomaticFinalOutputPaths` (struct, hors tests) `crates/whisper-core/src/ports.rs:364-368`; `AutomaticFinalImport` (struct, hors tests) `crates/whisper-core/src/ports.rs:373-378`; `AutomaticFinalTranscriptionStatus` (enum, hors tests) `crates/whisper-core/src/ports.rs:381-386`; `AutomaticFinalTranscriptionView` (struct, hors tests) `crates/whisper-core/src/ports.rs:389-397`; `ParakeetCaptureIdentity` (struct, hors tests) `crates/whisper-core/src/ports.rs:402-406`; `ParakeetCaptureViewState` (enum, hors tests) `crates/whisper-core/src/ports.rs:409-415`; `ParakeetCaptureView` (struct, hors tests) `crates/whisper-core/src/ports.rs:418-437`; `ImportQueueStatus` (enum, hors tests) `crates/whisper-core/src/ports.rs:440-445`; `DiagnosticPhase` (enum, hors tests) `crates/whisper-core/src/ports.rs:448-457`; `DiagnosticNotice` (enum, hors tests) `crates/whisper-core/src/ports.rs:460-466`; `ImportQueueEntry` (struct, hors tests) `crates/whisper-core/src/ports.rs:469-473`; `SourceSnapshotRecoveryCandidate` (struct, hors tests) `crates/whisper-core/src/ports.rs:477-483`; `ImportEffect` (enum, hors tests) `crates/whisper-core/src/ports.rs:486-589`; `ImportEvent` (enum, hors tests) `crates/whisper-core/src/ports.rs:592-838`; `ArchiveHistoryItem` (struct, hors tests) `crates/whisper-core/src/ports.rs:841-848`; `ReplacementCleanupInfo` (struct, hors tests) `crates/whisper-core/src/ports.rs:851-858`; `LiveRecoveryInfo` (struct, hors tests) `crates/whisper-core/src/ports.rs:861-868`; `ImportIoPort` (trait, hors tests) `crates/whisper-core/src/ports.rs:870-873`.

Fonctions avec corps : `replace_preview` `crates/whisper-core/src/ports.rs:197-204`; `read_preview` `crates/whisper-core/src/ports.rs:206-212`; `legacy_transcription_strategy` `crates/whisper-core/src/ports.rs:230-232`.

Déclarations `pub` (visibilité sous conditions) : `pub enum PortError {` `crates/whisper-core/src/ports.rs:6`; `pub trait Clock {` `crates/whisper-core/src/ports.rs:16`; `pub trait JobRepository {` `crates/whisper-core/src/ports.rs:20`; `pub struct PcmBlock {` `crates/whisper-core/src/ports.rs:26`; `pub struct SourceIdentity {` `crates/whisper-core/src/ports.rs:34`; `pub struct DecodeRequest {` `crates/whisper-core/src/ports.rs:41`; `pub struct DecodedPcmBlock {` `crates/whisper-core/src/ports.rs:50`; `pub trait DecoderPort {` `crates/whisper-core/src/ports.rs:57`; `pub struct WorkerRequest {` `crates/whisper-core/src/ports.rs:62`; `pub struct WorkerSegment {` `crates/whisper-core/src/ports.rs:71`; `pub trait WorkerPort {` `crates/whisper-core/src/ports.rs:80`; `pub struct ParakeetDocumentEntry {` `crates/whisper-core/src/ports.rs:88`; `pub struct ParakeetDocumentPage {` `crates/whisper-core/src/ports.rs:96`; `pub struct ParakeetPreview {` `crates/whisper-core/src/ports.rs:104`; `pub enum UnifiedSessionPhase {` `crates/whisper-core/src/ports.rs:110`; `pub struct UnifiedAudioPart {` `crates/whisper-core/src/ports.rs:120`; `pub struct UnifiedAudioPartPage {` `crates/whisper-core/src/ports.rs:128`; `pub struct UnifiedParakeetSessionView {` `crates/whisper-core/src/ports.rs:136`; `pub struct UnifiedParakeetSelectionView {` `crates/whisper-core/src/ports.rs:155`; `pub struct UnifiedPublicationFiles {` `crates/whisper-core/src/ports.rs:166`; `pub struct ParakeetWindowCoverage {` `crates/whisper-core/src/ports.rs:173`; `pub trait ParakeetDocumentPort {` `crates/whisper-core/src/ports.rs:179`; `pub struct ImportRequest {` `crates/whisper-core/src/ports.rs:216`; `pub enum DuplicateImportChoice {` `crates/whisper-core/src/ports.rs:235`; `pub struct AudioIdentity {` `crates/whisper-core/src/ports.rs:250`; `pub enum AudioFormat {` `crates/whisper-core/src/ports.rs:256`; `pub struct TreatmentIdentity {` `crates/whisper-core/src/ports.rs:264`; `pub struct OperationIdentity {` `crates/whisper-core/src/ports.rs:272`; `pub enum ReprocessingTarget {` `crates/whisper-core/src/ports.rs:280`; `pub enum ReprocessingStage {` `crates/whisper-core/src/ports.rs:288`; `pub struct ReprocessingProgress {` `crates/whisper-core/src/ports.rs:299`; `pub struct PublishedImport {` `crates/whisper-core/src/ports.rs:307`; `pub struct DuplicateImportPrompt {` `crates/whisper-core/src/ports.rs:321`; `pub struct LiveRequest {` `crates/whisper-core/src/ports.rs:344`; `pub struct AutomaticFinalOutputPaths {` `crates/whisper-core/src/ports.rs:364`; `pub struct AutomaticFinalImport {` `crates/whisper-core/src/ports.rs:373`; `pub enum AutomaticFinalTranscriptionStatus {` `crates/whisper-core/src/ports.rs:381`; `pub struct AutomaticFinalTranscriptionView {` `crates/whisper-core/src/ports.rs:389`; `pub struct ParakeetCaptureIdentity {` `crates/whisper-core/src/ports.rs:402`; `pub enum ParakeetCaptureViewState {` `crates/whisper-core/src/ports.rs:409`; `pub struct ParakeetCaptureView {` `crates/whisper-core/src/ports.rs:418`; `pub enum ImportQueueStatus {` `crates/whisper-core/src/ports.rs:440`; `pub enum DiagnosticPhase {` `crates/whisper-core/src/ports.rs:448`; `pub enum DiagnosticNotice {` `crates/whisper-core/src/ports.rs:460`; `pub struct ImportQueueEntry {` `crates/whisper-core/src/ports.rs:469`; `pub struct SourceSnapshotRecoveryCandidate {` `crates/whisper-core/src/ports.rs:477`; `pub enum ImportEffect {` `crates/whisper-core/src/ports.rs:486`; `pub enum ImportEvent {` `crates/whisper-core/src/ports.rs:592`; `pub struct ArchiveHistoryItem {` `crates/whisper-core/src/ports.rs:841`; `pub struct ReplacementCleanupInfo {` `crates/whisper-core/src/ports.rs:851`; `pub struct LiveRecoveryInfo {` `crates/whisper-core/src/ports.rs:861`; `pub trait ImportIoPort {` `crates/whisper-core/src/ports.rs:870`.

Dépend de, via `use crate::` : hors tests : `crate::domain::ComputeChoice` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain::Generation` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain::JobConfig` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain::JobId` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain::JobState` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain::SegmentId` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain::SourceRange` → `crate::domain` `crates/whisper-core/src/ports.rs:1`<br>hors tests : `crate::domain` → `crate::domain` `crates/whisper-core/src/ports.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::contracts` `crates/whisper-core/src/application/contracts.rs:2`<br>hors tests : `crate::application::event_metadata` `crates/whisper-core/src/application/event_metadata.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>hors tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:2`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>tests : `crate::application::runtime` `crates/whisper-core/src/application/runtime/mod.rs:7`<br>hors tests : `crate::application::runtime::parakeet_views` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::application::runtime::parakeet_views` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::application::runtime::parakeet_views` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::application::runtime::parakeet_views` `crates/whisper-core/src/application/runtime/parakeet_views.rs:2`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:2`<br>hors tests : `crate::ipc` `crates/whisper-core/src/ipc.rs:2`.

### integration::application_routing — `crates/whisper-core/tests/application_routing.rs`

Tests ou supports : définit `legacy_start_live_is_rejected_without_effects` `crates/whisper-core/tests/application_routing.rs:30-46`, `history_event_is_applied_without_an_active_job` `crates/whisper-core/tests/application_routing.rs:49-70`.

Mesure : 0 production / 70 tests / 70 physiques. `crates/whisper-core/tests/application_routing.rs:1-70`

Types : `State` (struct, tests) `crates/whisper-core/tests/application_routing.rs:10-13`; `MemoryPort` (struct, tests) `crates/whisper-core/tests/application_routing.rs:16`.

Fonctions avec corps : `submit` [tests] `crates/whisper-core/tests/application_routing.rs:19-22`; `poll` [tests] `crates/whisper-core/tests/application_routing.rs:24-26`; `legacy_start_live_is_rejected_without_effects` [tests] `crates/whisper-core/tests/application_routing.rs:30-46`; `history_event_is_applied_without_an_active_job` [tests] `crates/whisper-core/tests/application_routing.rs:49-70`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/application_routing.rs:1-70`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/application_routing.rs:1-70`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::compute_policy — `crates/whisper-core/tests/compute_policy.rs`

Tests ou supports : définit `forced_cpu_never_selects_or_initializes_gpu` `crates/whisper-core/tests/compute_policy.rs:6-15`, `auto_uses_gpu_when_available_and_allows_only_gpu_to_cpu_fallback` `crates/whisper-core/tests/compute_policy.rs:18-47`.

Mesure : 0 production / 63 tests / 63 physiques. `crates/whisper-core/tests/compute_policy.rs:1-63`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/compute_policy.rs:1-63`.

Fonctions avec corps : `forced_cpu_never_selects_or_initializes_gpu` [tests] `crates/whisper-core/tests/compute_policy.rs:6-15`; `auto_uses_gpu_when_available_and_allows_only_gpu_to_cpu_fallback` [tests] `crates/whisper-core/tests/compute_policy.rs:18-47`; `forced_gpu_failure_waits_for_a_human_choice` [tests] `crates/whisper-core/tests/compute_policy.rs:50-63`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/compute_policy.rs:1-63`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/compute_policy.rs:1-63`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::engine_capabilities — `crates/whisper-core/tests/engine_capabilities.rs`

Tests ou supports : définit `auto_and_manual_language_are_independent_and_projection_matches_admission` `crates/whisper-core/tests/engine_capabilities.rs:115-224`, `no_admissible_language_mode_refuses_launch_and_unknown_is_not_unsupported` `crates/whisper-core/tests/engine_capabilities.rs:227-291`.

Mesure : 0 production / 718 tests / 718 physiques. `crates/whisper-core/tests/engine_capabilities.rs:1-718`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/engine_capabilities.rs:1-718`.

Fonctions avec corps : `project` [tests] `crates/whisper-core/tests/engine_capabilities.rs:14-16`; `validate` [tests] `crates/whisper-core/tests/engine_capabilities.rs:18-20`; `key` [tests] `crates/whisper-core/tests/engine_capabilities.rs:22-36`; `state` [tests] `crates/whisper-core/tests/engine_capabilities.rs:38-43`; `language` [tests] `crates/whisper-core/tests/engine_capabilities.rs:45-51`; `definition` [tests] `crates/whisper-core/tests/engine_capabilities.rs:53-64`; `config` [tests] `crates/whisper-core/tests/engine_capabilities.rs:66-80`; `make_engine_snapshot` [tests] `crates/whisper-core/tests/engine_capabilities.rs:82-98`; `request` [tests] `crates/whisper-core/tests/engine_capabilities.rs:100-112`; `auto_and_manual_language_are_independent_and_projection_matches_admission` [tests] `crates/whisper-core/tests/engine_capabilities.rs:115-224`; `no_admissible_language_mode_refuses_launch_and_unknown_is_not_unsupported` [tests] `crates/whisper-core/tests/engine_capabilities.rs:227-291`; `turbo_qualification_does_not_admit_unqualified_parakeet_or_substitute_the_model` [tests] `crates/whisper-core/tests/engine_capabilities.rs:294-366`; `invalid_or_grayed_language_is_refused_even_when_injected_directly` [tests] `crates/whisper-core/tests/engine_capabilities.rs:369-415`; `preferences_are_retained_while_unavailable_models_remain_unlaunchable` [tests] `crates/whisper-core/tests/engine_capabilities.rs:418-476`; `unknown_or_incomparable_resources_do_not_prove_insufficiency_but_comparable_shortfall_does` [tests] `crates/whisper-core/tests/engine_capabilities.rs:479-569`; `comparable_shortfall_is_checked_for_ram_vram_and_model_disk` [tests] `crates/whisper-core/tests/engine_capabilities.rs:572-615`; `stale_generation_result_is_rejected_without_changing_current_preferences` [tests] `crates/whisper-core/tests/engine_capabilities.rs:618-689`; `existing_recognized_is_admissible_without_changing_the_l1a_qualified_api` [tests] `crates/whisper-core/tests/engine_capabilities.rs:692-718`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/engine_capabilities.rs:1-718`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/engine_capabilities.rs:1-718`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_inventory — `crates/whisper-core/tests/gpu_inventory.rs`

Tests ou supports : définit `inventory_keeps_homonymous_devices_and_excludes_confirmed_absence` `crates/whisper-core/tests/gpu_inventory.rs:36-49`, `current_device_with_missing_properties_is_kept_as_unknown` `crates/whisper-core/tests/gpu_inventory.rs:52-66`.

Mesure : 0 production / 400 tests / 400 physiques. `crates/whisper-core/tests/gpu_inventory.rs:1-400`

Types : `ControlledPort` (struct, tests) `crates/whisper-core/tests/gpu_inventory.rs:112`; `ControlledState` (struct, tests) `crates/whisper-core/tests/gpu_inventory.rs:115-118`; `QualifiedPort` (struct, tests) `crates/whisper-core/tests/gpu_inventory.rs:228`; `QualifiedPortState` (struct, tests) `crates/whisper-core/tests/gpu_inventory.rs:231-234`.

Fonctions avec corps : `observation` [tests] `crates/whisper-core/tests/gpu_inventory.rs:12-19`; `qualified` [tests] `crates/whisper-core/tests/gpu_inventory.rs:21-33`; `inventory_keeps_homonymous_devices_and_excludes_confirmed_absence` [tests] `crates/whisper-core/tests/gpu_inventory.rs:36-49`; `current_device_with_missing_properties_is_kept_as_unknown` [tests] `crates/whisper-core/tests/gpu_inventory.rs:52-66`; `unknown_or_conflicting_presence_is_not_reported_as_a_present_gpu` [tests] `crates/whisper-core/tests/gpu_inventory.rs:69-86`; `enumeration_error_after_observations_keeps_observations_and_marks_partial` [tests] `crates/whisper-core/tests/gpu_inventory.rs:89-96`; `completed_empty_inventory_is_distinct_from_partial_empty_inventory` [tests] `crates/whisper-core/tests/gpu_inventory.rs:99-109`; `start` [tests] `crates/whisper-core/tests/gpu_inventory.rs:121-124`; `poll` [tests] `crates/whisper-core/tests/gpu_inventory.rs:126-128`; `app_rejects_stale_responses_and_close_revokes_publication` [tests] `crates/whisper-core/tests/gpu_inventory.rs:132-158`; `app_publishes_partial_collection_and_failure_states` [tests] `crates/whisper-core/tests/gpu_inventory.rs:161-200`; `app_ignores_duplicate_terminal_response_until_a_new_generation_starts` [tests] `crates/whisper-core/tests/gpu_inventory.rs:203-225`; `start` [tests] `crates/whisper-core/tests/gpu_inventory.rs:237-240`; `poll` [tests] `crates/whisper-core/tests/gpu_inventory.rs:242-244`; `uses_qualified_catalogue` [tests] `crates/whisper-core/tests/gpu_inventory.rs:246-248`; `poll_qualified` [tests] `crates/whisper-core/tests/gpu_inventory.rs:250-252`; `app_publishes_qualified_partial_catalogue_and_rejects_late_or_duplicate_results` [tests] `crates/whisper-core/tests/gpu_inventory.rs:256-294`; `ready_and_disabled_states_are_preserved` [tests] `crates/whisper-core/tests/gpu_inventory.rs:297-307`; `gpu_inventory_types_are_plain_owned_domain_data` [tests] `crates/whisper-core/tests/gpu_inventory.rs:310-327`; `qualified_inventory_keeps_homonyms_and_sourced_windows_ranks` [tests] `crates/whisper-core/tests/gpu_inventory.rs:330-352`; `qualified_inventory_distinguishes_absent_and_conflicting_ranks_and_preserves_failure_cause` [tests] `crates/whisper-core/tests/gpu_inventory.rs:355-400`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/gpu_inventory.rs:1-400`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/gpu_inventory.rs:1-400`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_selection — `crates/whisper-core/tests/gpu_selection.rs`

Tests ou supports : définit `default_uses_windows_rank_even_when_motor_order_and_names_are_reversed` `crates/whisper-core/tests/gpu_selection.rs:45-76`, `preferred_compatible_device_wins_without_rank_and_missing_rank_never_becomes_default` `crates/whisper-core/tests/gpu_selection.rs:79-93`.

Mesure : 0 production / 297 tests / 297 physiques. `crates/whisper-core/tests/gpu_selection.rs:1-297`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/gpu_selection.rs:1-297`.

Fonctions avec corps : `row` [tests] `crates/whisper-core/tests/gpu_selection.rs:14-36`; `complete` [tests] `crates/whisper-core/tests/gpu_selection.rs:38-42`; `default_uses_windows_rank_even_when_motor_order_and_names_are_reversed` [tests] `crates/whisper-core/tests/gpu_selection.rs:45-76`; `preferred_compatible_device_wins_without_rank_and_missing_rank_never_becomes_default` [tests] `crates/whisper-core/tests/gpu_selection.rs:79-93`; `unavailable_preference_is_not_overwritten_and_substitution_uses_first_ranked_device` [tests] `crates/whisper-core/tests/gpu_selection.rs:96-112`; `incomplete_or_ambiguous_catalogue_waits_instead_of_claiming_no_gpu` [tests] `crates/whisper-core/tests/gpu_selection.rs:115-157`; `only_confirmed_identity_support_and_availability_make_a_gpu_selectable` [tests] `crates/whisper-core/tests/gpu_selection.rs:160-196`; `mode_and_preference_matrix_keeps_candidate_separate_from_effective_engine` [tests] `crates/whisper-core/tests/gpu_selection.rs:199-267`; `disabled_and_empty_identity_devices_are_never_gpu_candidates` [tests] `crates/whisper-core/tests/gpu_selection.rs:270-285`; `duplicate_windows_rank_cannot_choose_an_arbitrary_first_device` [tests] `crates/whisper-core/tests/gpu_selection.rs:288-297`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/gpu_selection.rs:1-297`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/gpu_selection.rs:1-297`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::import_contract — `crates/whisper-core/tests/import_contract.rs`

Tests ou supports : définit `application_keeps_bounded_strategy_in_prepare_and_start_effects` `crates/whisper-core/tests/import_contract.rs:49-72`, `application_publishes_only_after_end_and_synced_segment_receipt` `crates/whisper-core/tests/import_contract.rs:75-168`.

Mesure : 0 production / 902 tests / 902 physiques. `crates/whisper-core/tests/import_contract.rs:1-902`

Types : `FakeState` (struct, tests) `crates/whisper-core/tests/import_contract.rs:10-14`; `FakeIo` (struct, tests) `crates/whisper-core/tests/import_contract.rs:16`; `BoundedState` (struct, tests) `crates/whisper-core/tests/import_contract.rs:297-301`; `BoundedIo` (struct, tests) `crates/whisper-core/tests/import_contract.rs:303-306`; `BoundedState` (struct, tests) `crates/whisper-core/tests/import_contract.rs:452-456`; `BoundedIo` (struct, tests) `crates/whisper-core/tests/import_contract.rs:458-461`.

Fonctions avec corps : `submit` [tests] `crates/whisper-core/tests/import_contract.rs:18-24`; `poll` [tests] `crates/whisper-core/tests/import_contract.rs:25-27`; `request` [tests] `crates/whisper-core/tests/import_contract.rs:30-46`; `application_keeps_bounded_strategy_in_prepare_and_start_effects` [tests] `crates/whisper-core/tests/import_contract.rs:49-72`; `application_publishes_only_after_end_and_synced_segment_receipt` [tests] `crates/whisper-core/tests/import_contract.rs:75-168`; `refresh_drains_at_most_four_events_and_waits_for_every_persisted_ack` [tests] `crates/whisper-core/tests/import_contract.rs:171-293`; `persist_backpressure_retains_ninth_segment_until_queue_capacity_returns` [tests] `crates/whisper-core/tests/import_contract.rs:296-448`; `submit` [tests] `crates/whisper-core/tests/import_contract.rs:308-320`; `poll` [tests] `crates/whisper-core/tests/import_contract.rs:322-324`; `disconnected_persist_queue_is_fatal_and_does_not_retry_the_segment` [tests] `crates/whisper-core/tests/import_contract.rs:451-538`; `submit` [tests] `crates/whisper-core/tests/import_contract.rs:463-475`; `poll` [tests] `crates/whisper-core/tests/import_contract.rs:477-479`; `ready_and_following_failure_are_observable_on_separate_refreshes` [tests] `crates/whisper-core/tests/import_contract.rs:541-581`; `stop_invalidates_generation_and_rejects_late_segment_but_accepts_stop_receipt` [tests] `crates/whisper-core/tests/import_contract.rs:584-627`; `stop_during_preparation_and_after_end_never_completes_old_generation` [tests] `crates/whisper-core/tests/import_contract.rs:630-713`; `end_offset_and_segment_ranges_are_validated_before_publication` [tests] `crates/whisper-core/tests/import_contract.rs:716-778`; `failed_stop_receipt_is_visible_and_does_not_leave_cancelling_stuck` [tests] `crates/whisper-core/tests/import_contract.rs:781-805`; `invalid_wire_constructible_ranges_are_rejected_before_persistence` [tests] `crates/whisper-core/tests/import_contract.rs:808-860`; `refused_publish_never_becomes_complete` [tests] `crates/whisper-core/tests/import_contract.rs:863-902`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/import_contract.rs:1-902`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/import_contract.rs:1-902`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::live_contract — `crates/whisper-core/tests/live_contract.rs`

Tests ou supports : définit `starting_live_emits_one_explicit_effect_and_initializes_separate_counters` `crates/whisper-core/tests/live_contract.rs:47-76`, `forced_gpu_recovery_requires_an_explicit_cpu_finish_command` `crates/whisper-core/tests/live_contract.rs:79-109`.

Mesure : 0 production / 233 tests / 233 physiques. `crates/whisper-core/tests/live_contract.rs:1-233`

Types : `State` (struct, tests) `crates/whisper-core/tests/live_contract.rs:9-12`; `Fake` (struct, tests) `crates/whisper-core/tests/live_contract.rs:15`.

Fonctions avec corps : `submit` [tests] `crates/whisper-core/tests/live_contract.rs:18-21`; `poll` [tests] `crates/whisper-core/tests/live_contract.rs:23-25`; `request` [tests] `crates/whisper-core/tests/live_contract.rs:28-44`; `starting_live_emits_one_explicit_effect_and_initializes_separate_counters` [tests] `crates/whisper-core/tests/live_contract.rs:47-76`; `forced_gpu_recovery_requires_an_explicit_cpu_finish_command` [tests] `crates/whisper-core/tests/live_contract.rs:79-109`; `restored_live_recovery_is_visible_without_automatically_starting_a_new_passage` [tests] `crates/whisper-core/tests/live_contract.rs:112-139`; `explicit_live_publication_resolution_does_not_start_another_passage` [tests] `crates/whisper-core/tests/live_contract.rs:142-159`; `diagnostic_warning_is_visible_without_changing_job_state_and_stale_notice_is_ignored` [tests] `crates/whisper-core/tests/live_contract.rs:162-205`; `logging_degradation_remains_visible_after_subsequent_progress` [tests] `crates/whisper-core/tests/live_contract.rs:208-233`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/live_contract.rs:1-233`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/live_contract.rs:1-233`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::microphone_level — `crates/whisper-core/tests/microphone_level.rs`

Tests ou supports : définit `spectrum_context_binds_source_and_rejects_stale_foreign_and_future_updates` `crates/whisper-core/tests/microphone_level.rs:13-81`, `retry_barrier_rejects_same_owner_spectrum_acquired_before_cpu_retry` `crates/whisper-core/tests/microphone_level.rs:84-140`.

Mesure : 0 production / 238 tests / 238 physiques. `crates/whisper-core/tests/microphone_level.rs:1-238`

Types : `DefaultChangingPort` (struct, tests) `crates/whisper-core/tests/microphone_level.rs:7-10`.

Fonctions avec corps : `spectrum_context_binds_source_and_rejects_stale_foreign_and_future_updates` [tests] `crates/whisper-core/tests/microphone_level.rs:13-81`; `retry_barrier_rejects_same_owner_spectrum_acquired_before_cpu_retry` [tests] `crates/whisper-core/tests/microphone_level.rs:84-140`; `list_inputs` [tests] `crates/whisper-core/tests/microphone_level.rs:143-145`; `default_input_identity` [tests] `crates/whisper-core/tests/microphone_level.rs:147-149`; `gain_validation_and_neutral_conversion_match_the_microphone_contract` [tests] `crates/whisper-core/tests/microphone_level.rs:153-160`; `command_context_rejects_old_selection_and_keeps_same_named_devices_distinct` [tests] `crates/whisper-core/tests/microphone_level.rs:163-184`; `absent_measurement_is_not_a_valid_silent_measurement` [tests] `crates/whisper-core/tests/microphone_level.rs:187-201`; `windows_default_identity_resolves_before_open_and_is_refreshed_explicitly` [tests] `crates/whisper-core/tests/microphone_level.rs:204-238`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/microphone_level.rs:1-238`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/microphone_level.rs:1-238`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::model_catalog — `crates/whisper-core/tests/model_catalog.rs`

Tests ou supports : définit `catalog_is_closed_to_the_two_planned_model_identities` `crates/whisper-core/tests/model_catalog.rs:61-68`, `configuration_identity_includes_every_qualification_dimension` `crates/whisper-core/tests/model_catalog.rs:71-115`.

Mesure : 0 production / 242 tests / 242 physiques. `crates/whisper-core/tests/model_catalog.rs:1-242`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/model_catalog.rs:1-242`.

Fonctions avec corps : `project` [tests] `crates/whisper-core/tests/model_catalog.rs:12-14`; `validate` [tests] `crates/whisper-core/tests/model_catalog.rs:16-18`; `key` [tests] `crates/whisper-core/tests/model_catalog.rs:20-39`; `model` [tests] `crates/whisper-core/tests/model_catalog.rs:41-48`; `qualified_capability` [tests] `crates/whisper-core/tests/model_catalog.rs:50-58`; `catalog_is_closed_to_the_two_planned_model_identities` [tests] `crates/whisper-core/tests/model_catalog.rs:61-68`; `configuration_identity_includes_every_qualification_dimension` [tests] `crates/whisper-core/tests/model_catalog.rs:71-115`; `availability_states_keep_both_catalog_entries_and_disable_unavailable_models` [tests] `crates/whisper-core/tests/model_catalog.rs:118-162`; `a_qualification_for_one_configuration_does_not_transfer_to_another_revision` [tests] `crates/whisper-core/tests/model_catalog.rs:165-198`; `qualification_does_not_transfer_to_a_different_worker_build` [tests] `crates/whisper-core/tests/model_catalog.rs:201-242`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/model_catalog.rs:1-242`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/model_catalog.rs:1-242`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::model_preferences — `crates/whisper-core/tests/model_preferences.rs`

Tests ou supports : définit `turbo_is_default_and_user_model_and_auto_preferences_are_saved_with_revisions` `crates/whisper-core/tests/model_preferences.rs:51-71`, `failed_or_stale_preference_ack_is_reported_without_reverting_the_session_choice` `crates/whisper-core/tests/model_preferences.rs:74-99`.

Mesure : 0 production / 120 tests / 120 physiques. `crates/whisper-core/tests/model_preferences.rs:1-120`

Types : `MemoryPreferences` (struct, tests) `crates/whisper-core/tests/model_preferences.rs:10-14`.

Fonctions avec corps : `load` [tests] `crates/whisper-core/tests/model_preferences.rs:17-19`; `save` [tests] `crates/whisper-core/tests/model_preferences.rs:21-47`; `turbo_is_default_and_user_model_and_auto_preferences_are_saved_with_revisions` [tests] `crates/whisper-core/tests/model_preferences.rs:51-71`; `failed_or_stale_preference_ack_is_reported_without_reverting_the_session_choice` [tests] `crates/whisper-core/tests/model_preferences.rs:74-99`; `unknown_v1_model_path_is_preserved_as_a_diagnostic` [tests] `crates/whisper-core/tests/model_preferences.rs:102-120`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/model_preferences.rs:1-120`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/model_preferences.rs:1-120`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::model_selection_app — `crates/whisper-core/tests/model_selection_app.rs`

Tests ou supports : définit `turbo_existing_matrix_admits_all_1206_inputs_without_native_inference` `crates/whisper-core/tests/model_selection_app.rs:20-61`, `turbo_existing_matrix_admits_and_submits_all_1206_guard_requests` `crates/whisper-core/tests/model_selection_app.rs:64-144`.

Mesure : 0 production / 430 tests / 430 physiques. `crates/whisper-core/tests/model_selection_app.rs:1-430`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/model_selection_app.rs:1-430`.

Fonctions avec corps : `app` [tests] `crates/whisper-core/tests/model_selection_app.rs:11-17`; `turbo_existing_matrix_admits_all_1206_inputs_without_native_inference` [tests] `crates/whisper-core/tests/model_selection_app.rs:20-61`; `turbo_existing_matrix_admits_and_submits_all_1206_guard_requests` [tests] `crates/whisper-core/tests/model_selection_app.rs:64-144`; `existing_recognized_capability_outside_the_historical_matrix_stays_refused` [tests] `crates/whisper-core/tests/model_selection_app.rs:147-217`; `turbo_snapshot_with_only_a_new_existing_recognized_configuration_is_refused` [tests] `crates/whisper-core/tests/model_selection_app.rs:220-267`; `decode_sha256` [tests] `crates/whisper-core/tests/model_selection_app.rs:269-275`; `qualified_parakeet_cpu_gpu_auto_and_invalid_language_selections_are_checked` [tests] `crates/whisper-core/tests/model_selection_app.rs:278-368`; `parakeet_live_is_admitted_only_through_the_live_guard` [tests] `crates/whisper-core/tests/model_selection_app.rs:371-416`; `known_historical_paths_are_case_and_separator_insensitive_but_exact` [tests] `crates/whisper-core/tests/model_selection_app.rs:419-430`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/model_selection_app.rs:1-430`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/model_selection_app.rs:1-430`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_import_contract — `crates/whisper-core/tests/parakeet_import_contract.rs`

Tests ou supports : définit `only_qualified_cpu_import_with_automatic_language_is_admitted` `crates/whisper-core/tests/parakeet_import_contract.rs:71-119`, `parakeet_import_admits_long_product_input_but_rejects_wrong_hash` `crates/whisper-core/tests/parakeet_import_contract.rs:122-136`.

Mesure : 0 production / 278 tests / 278 physiques. `crates/whisper-core/tests/parakeet_import_contract.rs:1-278`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/parakeet_import_contract.rs:1-278`.

Fonctions avec corps : `qualified_guard` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:12-53`; `import` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:55-68`; `only_qualified_cpu_import_with_automatic_language_is_admitted` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:71-119`; `parakeet_import_admits_long_product_input_but_rejects_wrong_hash` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:122-136`; `parakeet_duration_selects_adapter_profile_not_core_admission` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:139-153`; `current_r6_cpu_and_l3b_cuda_test_keys_are_qualified` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:156-213`; `personal_release_keys_match_the_qualified_candidate_b_workers_and_dll_sets` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:216-244`; `turbo_live_guard_remains_available_through_legacy_constructor` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:247-270`; `decode_sha256` [tests] `crates/whisper-core/tests/parakeet_import_contract.rs:272-278`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/parakeet_import_contract.rs:1-278`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/parakeet_import_contract.rs:1-278`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_ipc — `crates/whisper-core/tests/parakeet_ipc.rs`

Tests ou supports : définit `parakeet_protocol_is_versioned_and_caps_are_explicit` `crates/whisper-core/tests/parakeet_ipc.rs:8-13`, `transcribe_command_carries_complete_build_runtime_profile_and_attempt_identity` `crates/whisper-core/tests/parakeet_ipc.rs:16-49`.

Mesure : 0 production / 113 tests / 113 physiques. `crates/whisper-core/tests/parakeet_ipc.rs:1-113`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/parakeet_ipc.rs:1-113`.

Fonctions avec corps : `parakeet_protocol_is_versioned_and_caps_are_explicit` [tests] `crates/whisper-core/tests/parakeet_ipc.rs:8-13`; `transcribe_command_carries_complete_build_runtime_profile_and_attempt_identity` [tests] `crates/whisper-core/tests/parakeet_ipc.rs:16-49`; `result_contract_keeps_request_identity_and_owned_word_times` [tests] `crates/whisper-core/tests/parakeet_ipc.rs:52-104`; `test_identity` [tests] `crates/whisper-core/tests/parakeet_ipc.rs:106-113`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/parakeet_ipc.rs:1-113`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/parakeet_ipc.rs:1-113`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_windows — `crates/whisper-core/tests/parakeet_windows.rs`

Tests ou supports : définit `silence_threshold_and_minimum_duration_are_inclusive` `crates/whisper-core/tests/parakeet_windows.rs:18-31`, `chooses_earliest_of_equal_distance_silences_near_ninety_seconds` `crates/whisper-core/tests/parakeet_windows.rs:34-45`.

Mesure : 0 production / 194 tests / 194 physiques. `crates/whisper-core/tests/parakeet_windows.rs:1-194`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-core/tests/parakeet_windows.rs:1-194`.

Fonctions avec corps : `audio` [tests] `crates/whisper-core/tests/parakeet_windows.rs:8-10`; `add_silence` [tests] `crates/whisper-core/tests/parakeet_windows.rs:12-15`; `silence_threshold_and_minimum_duration_are_inclusive` [tests] `crates/whisper-core/tests/parakeet_windows.rs:18-31`; `chooses_earliest_of_equal_distance_silences_near_ninety_seconds` [tests] `crates/whisper-core/tests/parakeet_windows.rs:34-45`; `extends_after_one_hundred_twenty_seconds_to_the_next_silence` [tests] `crates/whisper-core/tests/parakeet_windows.rs:48-57`; `eof_short_tail_and_integral_silence_are_processed_without_forced_cuts` [tests] `crates/whisper-core/tests/parakeet_windows.rs:60-67`; `no_silence_before_native_bound_returns_an_explicit_error` [tests] `crates/whisper-core/tests/parakeet_windows.rs:70-76`; `reference_profile_duration_is_bounded_and_window_margins_are_joint` [tests] `crates/whisper-core/tests/parakeet_windows.rs:79-106`; `timestamps_use_cumulative_sample_offsets_and_reject_internal_overflow` [tests] `crates/whisper-core/tests/parakeet_windows.rs:109-144`; `only_the_final_file_word_can_be_capped_by_one_tdt_frame` [tests] `crates/whisper-core/tests/parakeet_windows.rs:147-194`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/parakeet_windows.rs:1-194`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/parakeet_windows.rs:1-194`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::scheduler_contract — `crates/whisper-core/tests/scheduler_contract.rs`

Tests ou supports : définit `admission_and_restoration_never_start_a_worker_without_user_choice` `crates/whisper-core/tests/scheduler_contract.rs:43-77`, `enqueue_is_acknowledged_only_after_the_durable_queue_event` `crates/whisper-core/tests/scheduler_contract.rs:80-114`.

Mesure : 0 production / 154 tests / 154 physiques. `crates/whisper-core/tests/scheduler_contract.rs:1-154`

Types : `State` (struct, tests) `crates/whisper-core/tests/scheduler_contract.rs:9-12`; `Fake` (struct, tests) `crates/whisper-core/tests/scheduler_contract.rs:14`.

Fonctions avec corps : `submit` [tests] `crates/whisper-core/tests/scheduler_contract.rs:16-19`; `poll` [tests] `crates/whisper-core/tests/scheduler_contract.rs:20-22`; `request` [tests] `crates/whisper-core/tests/scheduler_contract.rs:24-40`; `admission_and_restoration_never_start_a_worker_without_user_choice` [tests] `crates/whisper-core/tests/scheduler_contract.rs:43-77`; `enqueue_is_acknowledged_only_after_the_durable_queue_event` [tests] `crates/whisper-core/tests/scheduler_contract.rs:80-114`; `enqueue_failure_is_visible_without_marking_a_job_active` [tests] `crates/whisper-core/tests/scheduler_contract.rs:117-131`; `interrupted_resume_requires_an_explicit_command_and_advances_generation` [tests] `crates/whisper-core/tests/scheduler_contract.rs:134-154`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-core/tests/scheduler_contract.rs:1-154`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-core/tests/scheduler_contract.rs:1-154`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 0 | 0 |
| cfg_feature | 0 | 0 |
| cfg_test | 0 | 11 |
| command_new | 0 | 0 |
| expect | 5 | 4 |
| thread_spawn | 0 | 0 |
| unsafe | 0 | 0 |
| unwrap | 10 | 483 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 24 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 10 sur plages hors tests et 14 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::assertions_on_constants | 0 | 1 |
| clippy::cognitive_complexity | 0 | 2 |
| clippy::field_reassign_with_default | 0 | 1 |
| clippy::too_many_lines | 8 | 10 |
| dead_code | 2 | 0 |
