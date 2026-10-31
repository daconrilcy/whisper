Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-adapters

## Rôle et type

Exécute les effets avec capture, archives et transport vers les workers. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237` ; `crates/whisper-adapters/src/capture.rs:1-1186` ; `crates/whisper-adapters/src/archive/publication.rs:1-291` ; `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1-812`

AsyncImportIo échange effets et événements avec des services sur threads séparés. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237` ; `crates/whisper-adapters/src/capture.rs:1-1186` ; `crates/whisper-adapters/src/archive/publication.rs:1-291` ; `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1-812`

Type : **Bibliothèque d’adaptateurs**. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237` ; `crates/whisper-adapters/src/capture.rs:1-1186` ; `crates/whisper-adapters/src/archive/publication.rs:1-291` ; `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1-812`

Mesures : **35016 production / 45547 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-adapters/Cargo.toml:17` |
| serde | externe | dependencies | {"version": "=1.0.228", "features": ["derive"]} | `crates/whisper-adapters/Cargo.toml:18` |
| serde_json | externe | dependencies | {"workspace": true} | `crates/whisper-adapters/Cargo.toml:19` |
| sha2 | externe | dependencies | "=0.10.9" | `crates/whisper-adapters/Cargo.toml:20` |
| same-file | externe | dependencies | "=1.0.6" | `crates/whisper-adapters/Cargo.toml:21` |
| symphonia | externe | dependencies | {"version": "=0.5.5", "default-features": false, "features": ["wav", "pcm", "mp3"]} | `crates/whisper-adapters/Cargo.toml:22` |
| cpal | externe | dependencies | {"version": "=0.18.2", "default-features": false} | `crates/whisper-adapters/Cargo.toml:23` |
| rubato | externe | dependencies | {"version": "=0.16.2", "default-features": false, "features": ["fft_resampler"]} | `crates/whisper-adapters/Cargo.toml:24` |
| webrtc-vad | externe | dependencies | "=0.4.0" | `crates/whisper-adapters/Cargo.toml:25` |
| rtrb | externe | dependencies | "=0.4.0" | `crates/whisper-adapters/Cargo.toml:26` |
| rustfft | externe | dependencies | "=6.4.1" | `crates/whisper-adapters/Cargo.toml:27` |
| rfd | externe | cfg(target_os = "windows") / dependencies | {"version": "=0.17.2", "default-features": false} | `crates/whisper-adapters/Cargo.toml:30` |
| wmi | externe | cfg(target_os = "windows") / dependencies | "=0.18.4" | `crates/whisper-adapters/Cargo.toml:31` |
| windows | externe | cfg(target_os = "windows") / dependencies | {"version": "=0.62.2", "features": ["Win32_Foundation", "Win32_Storage_FileSystem", "Win32_Graphics_Dxgi", "Win32_Graphics_Dxgi_Common", "Win32_Devices_Display", "Win32_Devices_DeviceAndDriverInstallation"]} | `crates/whisper-adapters/Cargo.toml:32` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-adapters/Cargo.toml:1-38`.

## Interfaces publiques et usages intercrates

| Consommateur | Contexte | Import observé | Source |
| --- | --- | --- | --- |
| whisper-desktop | hors tests / cfg | `use whisper_adapters::model_catalog::CatalogSnapshot;` | `crates/whisper-desktop/src/root.rs:6` |
| whisper-desktop | hors tests / cfg | `use whisper_adapters::{ gpu_inventory::WindowsGpuInventoryPort, model_catalog::LocalModelCatalog, path_settings::WindowsModelPreferencePort, path_settings::WindowsPathPort, supervisor::WorkerPaths, worker_ipc::{AsyncImportIo, probe_cuda_worker}, };` | `crates/whisper-desktop/src/root.rs:7-14` |
| whisper-desktop | tests | `use whisper_adapters::parakeet_import::ParakeetWorkerPaths;` | `crates/whisper-desktop/src/root.rs:1146` |
| whisper-desktop | hors tests / cfg | `use whisper_adapters::model_catalog::CatalogSnapshot;` | `crates/whisper-desktop/src/ui/model.rs:9` |
| whisper-desktop | hors tests / cfg | `use whisper_adapters::model_catalog::CatalogSnapshot;` | `crates/whisper-desktop/src/ui/presentation.rs:3` |
| whisper-desktop | tests | `use whisper_adapters::{ capture::ProbeAudioBlock, microphone_probe::{MicrophoneProbe, MicrophoneSpectrumReader}, };` | `crates/whisper-desktop/src/ui/tests/support.rs:343-346` |

Interfaces principales avec définitions et sites d’usage (pour desktop, entre cibles du même package) :

| Interface | Définition | Usage observé |
| --- | --- | --- |
| `AsyncImportIo` | `crates/whisper-adapters/src/worker_ipc/service/mod.rs:25-41` | `crates/whisper-desktop/src/root.rs:43-319` |
| `WorkerPaths` | `crates/whisper-adapters/src/supervisor.rs:29-32` | `crates/whisper-desktop/src/root.rs:43-319` |
| `WasapiMicrophonePort` | `crates/whisper-adapters/src/capture.rs:342` | `crates/whisper-desktop/src/main.rs:5-33` |
| `LocalModelCatalog` | `crates/whisper-adapters/src/model_catalog.rs:238-240` | `crates/whisper-desktop/src/root.rs:43-319` |
| `probe_cuda_worker` | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:21-52` | `crates/whisper-desktop/src/root.rs:43-319` |

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-adapters/src/lib.rs:1-34`. Imports textuels, pas un graphe de symboles résolu.

Cible de build partagée : `scripts/recette-build.rs:1-7` (7 lignes hors tests, comptées une seule fois hors des totaux par crate). Elle refuse recette-hooks en release. Déclaration : `crates/whisper-adapters/Cargo.toml:7`.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | `crates/whisper-adapters/src/archive/campaign.rs:91` ; `crates/whisper-adapters/src/archive/cleanup/execution.rs:230` ; `crates/whisper-adapters/src/archive/cleanup/proofs.rs:37` ; `crates/whisper-adapters/src/archive/cleanup.rs:25` ; `crates/whisper-adapters/src/archive/identity.rs:220` ; `crates/whisper-adapters/src/archive/import.rs:29` ; `crates/whisper-adapters/src/archive/journal.rs:28` ; `crates/whisper-adapters/src/archive/live.rs:19` ; `crates/whisper-adapters/src/archive/publication/catalog.rs:46` ; `crates/whisper-adapters/src/archive/publication/exports.rs:16` ; `crates/whisper-adapters/src/archive/publication.rs:111` ; `crates/whisper-adapters/src/archive/recovery/live.rs:26` ; `crates/whisper-adapters/src/archive/recovery.rs:57` ; `crates/whisper-adapters/src/archive/source.rs:39` ; `crates/whisper-adapters/src/archive/storage.rs:13` ; `crates/whisper-adapters/src/archive.rs:4` ; `crates/whisper-adapters/src/direct_live/outputs.rs:38` ; `crates/whisper-adapters/src/direct_live/wav.rs:21` ; `crates/whisper-adapters/src/direct_live.rs:31` ; `crates/whisper-adapters/src/gpu_inventory.rs:1` ; `crates/whisper-adapters/src/model_catalog.rs:2` ; `crates/whisper-adapters/src/parakeet_audio.rs:7` ; `crates/whisper-adapters/src/parakeet_document.rs:4` ; `crates/whisper-adapters/src/parakeet_import.rs:9` ; `crates/whisper-adapters/src/parakeet_live/events.rs:240` ; `crates/whisper-adapters/src/parakeet_live/inference.rs:273` ; `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:85` ; `crates/whisper-adapters/src/parakeet_live/windows.rs:163` ; `crates/whisper-adapters/src/parakeet_live.rs:21` ; `crates/whisper-adapters/src/path_settings.rs:2` ; `crates/whisper-adapters/src/queue_store.rs:5` ; `crates/whisper-adapters/src/source_binding.rs:4` ; `crates/whisper-adapters/src/staging.rs:3` ; `crates/whisper-adapters/src/supervisor.rs:3` ; `crates/whisper-adapters/src/windows_file_identity.rs:9` ; `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:169` ; `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:917` ; `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:122` ; `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:117` ; `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:339` ; `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:80` ; `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:110` ; `crates/whisper-adapters/src/worker_ipc/support.rs:4` |
| Processus | `crates/whisper-adapters/src/parakeet_import.rs:411` ; `crates/whisper-adapters/src/parakeet_live/inference.rs:170` ; `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:465` ; `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:57` ; `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:541` ; `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:377` |
| Threads/canaux | `crates/whisper-adapters/src/archive/campaign.rs:471` ; `crates/whisper-adapters/src/decoder.rs:46` ; `crates/whisper-adapters/src/gpu_inventory.rs:6` ; `crates/whisper-adapters/src/microphone_probe.rs:6` ; `crates/whisper-adapters/src/parakeet_audio.rs:10` ; `crates/whisper-adapters/src/parakeet_import.rs:7` ; `crates/whisper-adapters/src/parakeet_live/capture.rs:314` ; `crates/whisper-adapters/src/parakeet_live/inference.rs:164` ; `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:88` ; `crates/whisper-adapters/src/parakeet_live.rs:24` ; `crates/whisper-adapters/src/path_settings.rs:5` ; `crates/whisper-adapters/src/staging.rs:9` ; `crates/whisper-adapters/src/supervisor.rs:9` ; `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:47` ; `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:19` ; `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:11` ; `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:217` ; `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:90` ; `crates/whisper-adapters/src/worker_ipc/service/mod.rs:15` |
| Entrées/sorties standard | `crates/whisper-adapters/src/parakeet_import.rs:455` ; `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:65` |
| GPU/natif | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate::archive::campaign | Contient barrières et marqueurs de campagne conditionnés par recette-hooks. `crates/whisper-adapters/src/archive/campaign.rs:1-484` | 484 | 0 |
| crate::archive::cleanup::execution | Exécute les suppressions de remplacement après contrôle des identités et chemins. `crates/whisper-adapters/src/archive/cleanup/execution.rs:1-561` | 561 | 0 |
| crate::archive::cleanup::proofs | Relève les identités des fichiers et conteneurs contrôlés lors du retrait. `crates/whisper-adapters/src/archive/cleanup/proofs.rs:1-569` | 569 | 0 |
| crate::archive::cleanup | Lit/écrit les plans de retrait des traitements remplacés et permet leur reprise. `crates/whisper-adapters/src/archive/cleanup.rs:1-220` | 220 | 0 |
| crate::archive::identity | Valide identités/artefacts d’archives et lit les pages document/audio unifiées. `crates/whisper-adapters/src/archive/identity.rs:1-877` | 877 | 0 |
| crate::archive::import | Prépare les générations import, sources segmentées et confirmations VAD. `crates/whisper-adapters/src/archive/import.rs:1-517` | 517 | 0 |
| crate::archive::journal | Persiste et relit les segments et valide le préfixe de reprise. `crates/whisper-adapters/src/archive/journal.rs:1-308` | 308 | 0 |
| crate::archive::live | Persiste paquets MP3, tentatives et fenêtres live puis publie les artefacts. `crates/whisper-adapters/src/archive/live.rs:1-714` | 714 | 0 |
| crate::archive::publication::catalog | Gère index, réservations, doublons et intentions de publication. `crates/whisper-adapters/src/archive/publication/catalog.rs:1-1082` | 1082 | 0 |
| crate::archive::publication::exports | Installe les exports et vérifie leur propriété et identité de fichier. `crates/whisper-adapters/src/archive/publication/exports.rs:1-623` | 615 | 8 |
| crate::archive::publication | Prépare puis engage la publication d’une génération import. `crates/whisper-adapters/src/archive/publication.rs:1-291` | 287 | 4 |
| crate::archive::recovery::live | Scanne générations live/préfixes confirmés et réconcilie les alias. `crates/whisper-adapters/src/archive/recovery/live.rs:1-405` | 405 | 0 |
| crate::archive::recovery | Scanne les générations et prépare la reprise depuis un préfixe vérifié. `crates/whisper-adapters/src/archive/recovery.rs:1-752` | 752 | 0 |
| crate::archive::source | Identifie et empreinte les sources simples/segmentées et mesure leur durée. `crates/whisper-adapters/src/archive/source.rs:1-198` | 198 | 0 |
| crate::archive::storage | Écrit, copie et synchronise des fichiers nouveaux ou remplacés. `crates/whisper-adapters/src/archive/storage.rs:1-66` | 66 | 0 |
| crate::archive::tests | Tests ou supports : définit `recette_policy_l04_requires_the_feature` `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:6-14`, `bounded_resume_accepts_a_current_backend_for_the_same_model_profile` `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:17-27`. | 0 | 715 |
| crate::archive::tests | Tests ou supports : définit `l04_campaign_gate_rejects_escape_and_duplicate_trigger` `crates/whisper-adapters/src/archive/tests/campaign.rs:3-146`. | 0 | 146 |
| crate::archive::tests | Tests ou supports : définit `archive_characterization_truncated_cleanup_plan_is_refused_without_disposition` `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:2-26`. | 0 | 26 |
| crate::archive::tests | Tests ou supports : définit `archive_characterization_ownership_versions_one_two_three_follow_existing_paths` `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:14-55`, `archive_characterization_ownership_truncated_or_unknown_refuses_without_mutation` `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:58-94`. | 0 | 262 |
| crate::archive::tests | Tests ou supports : définit `archive_characterization_stream_error_keeps_final_and_retry_replaces_it` `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:78-100`, `archive_characterization_replace_obstacle_keeps_temporary_until_retry` `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:103-126`. | 0 | 335 |
| crate::archive | Tests ou supports : définit `inject_copy_fallback_publication_cut` `crates/whisper-adapters/src/archive/tests/publication_support.rs:20-25`, `clear_copy_fallback_publication_cut` `crates/whisper-adapters/src/archive/tests/publication_support.rs:28-32`. | 0 | 59 |
| crate::archive::tests | Tests ou supports : définit `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` `crates/whisper-adapters/src/archive/tests/recovery.rs:2-327`, `segment_recovery_keeps_synced_prefix_and_drops_only_a_partial_tail` `crates/whisper-adapters/src/archive/tests/recovery.rs:330-351`. | 0 | 847 |
| crate::archive::tests | Tests ou supports : définit `archive_characterization_pending_absent_truncated_and_divergent` `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:2-40`, `archive_characterization_segment_journal_repairs_initial_and_large_partial_tail` `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:43-64`. | 0 | 85 |
| crate::archive::tests | Tests ou supports : définit `r11_replace_deactivates_every_matching_publication_and_keeps_other_audio` `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:2-87`, `r11_create_new_preserves_existing_publications_and_index_write_is_atomic` `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:90-135`. | 0 | 413 |
| crate::archive::tests | Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-adapters/src/archive/tests.rs:1-8`. | 0 | 8 |
| crate::archive | Définit ArchiveStore/LiveArchive et distribue leurs implémentations dans les sous-modules. `crates/whisper-adapters/src/archive.rs:1-581` | 566 | 15 |
| crate::capture | Ouvre les entrées microphone et convertit les blocs avec compensation et gain. `crates/whisper-adapters/src/capture.rs:1-1186` | 691 | 495 |
| crate::decoder | Valide une demande de décodage et la transmet par WorkerTransport. `crates/whisper-adapters/src/decoder.rs:1-251` | 103 | 148 |
| crate::direct_live::capture | Draine les blocs capturés et publie les mesures niveau/spectre. `crates/whisper-adapters/src/direct_live/capture.rs:1-148` | 148 | 0 |
| crate::direct_live::d2_corpus | Tests ou supports : définit `d2_corpus_real_cpu_turbo_and_parakeet` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:143-282`, `d2_frame_deadlines_follow_the_monotone_one_x_sample_clock` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:455-468`. | 0 | 622 |
| crate::direct_live::outputs | Écrit les exports texte/SRT et les manifestes capture/finalisation Parakeet. `crates/whisper-adapters/src/direct_live/outputs.rs:1-267` | 267 | 0 |
| crate::direct_live::test_support | Tests ou supports : définit `from_test_capture` `crates/whisper-adapters/src/direct_live/test_support.rs:5-53`, `from_test_capture_with_cpu_worker` `crates/whisper-adapters/src/direct_live/test_support.rs:56-110`. | 0 | 129 |
| crate::direct_live::tests | Tests ou supports : définit `direct_capture_publishes_frequency_spectrum_without_changing_wav_audio` `crates/whisper-adapters/src/direct_live/tests.rs:26-112`, `direct_capture_poll_keeps_spectrum_text_and_terminal_progress_within_four_events` `crates/whisper-adapters/src/direct_live/tests.rs:115-209`. | 0 | 1262 |
| crate::direct_live::wav | Écrit le WAV de capture et relit les plages de fenêtres en attente. `crates/whisper-adapters/src/direct_live/wav.rs:1-134` | 134 | 0 |
| crate::direct_live::worker | Traite les événements du worker direct et le repli CPU avant disponibilité. `crates/whisper-adapters/src/direct_live/worker.rs:1-336` | 336 | 0 |
| crate::direct_live | Porte la session micro Turbo et pilote capture, worker, fenêtres et export. `crates/whisper-adapters/src/direct_live.rs:1-470` | 458 | 12 |
| crate::gpu_inventory::windows_native | Énumère les adaptateurs DXGI et leurs identifiants Windows. `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:1-161` | 139 | 22 |
| crate::gpu_inventory | Collecte les observations Windows/GPU et les preuves du worker. `crates/whisper-adapters/src/gpu_inventory.rs:1-855` | 613 | 242 |
| crate::journal | Réexporte SegmentRecord depuis archive. `crates/whisper-adapters/src/journal.rs:1-3` | 3 | 0 |
| crate | Déclare les modules et interfaces de bibliothèque. `crates/whisper-adapters/src/lib.rs:1-34` | 34 | 0 |
| crate::microphone_probe::retry_epoch_regression | Tests ou supports : définit `full_queue_invalidates_old_epoch_before_fresh_contiguous_audio` `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:17-69`. | 0 | 69 |
| crate::microphone_probe | Calcule niveaux/spectres dans un thread avec contrôle d’époque. `crates/whisper-adapters/src/microphone_probe.rs:1-1252` | 690 | 562 |
| crate::model_catalog | Inspecte les modèles locaux et met en cache les empreintes de leurs fichiers. `crates/whisper-adapters/src/model_catalog.rs:1-549` | 455 | 94 |
| crate::parakeet_audio | Écrit les parties audio et acquittements et décode les imports en parties. `crates/whisper-adapters/src/parakeet_audio.rs:1-1061` | 861 | 200 |
| crate::parakeet_live::confirmation | Compare les hypothèses de mots pour produire blocs confirmés et plages restantes. `crates/whisper-adapters/src/parakeet_confirmation.rs:1-1398` | 544 | 854 |
| crate::parakeet_document | Persiste segments finaux/couvertures/previews et sert les pages du document. `crates/whisper-adapters/src/parakeet_document.rs:1-1066` | 736 | 330 |
| crate::parakeet_import::historical_runtime | Vérifie les chemins de runtimes et références historiques de qualification. `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:1-364` | 364 | 0 |
| crate::parakeet_import::legacy_runtime_tests | Tests ou supports : définit `personal_cpu_and_gpu_do_not_require_l3b` `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:27-76`, `historical_cpu_requires_exact_worker_configuration_and_hash` `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:79-124`. | 0 | 124 |
| crate::parakeet_import::runtime_isolation_tests | Tests ou supports : définit `personal_session_is_independent_of_historical_environment` `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:7-9`. | 0 | 9 |
| crate::parakeet_import | Vérifie les ressources Parakeet et contrôle le processus et ses échanges IPC. `crates/whisper-adapters/src/parakeet_import.rs:1-937` | 853 | 84 |
| crate::parakeet_live::capture | Gère capture, gain, parties audio et acquittements du micro Parakeet. `crates/whisper-adapters/src/parakeet_live/capture.rs:1-455` | 432 | 23 |
| crate::parakeet_live::diagnostics_absence_tests | Tests ou supports : définit `legacy_environment_does_not_create_diagnostic_files` `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:8-56`, `d2_test_diagnostics_remain_available_in_guarded_destination` `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:59-94`. | 0 | 94 |
| crate::parakeet_live::events | Draine les résultats de fenêtres et produit les événements progression/audio. `crates/whisper-adapters/src/parakeet_live/events.rs:1-280` | 238 | 42 |
| crate::parakeet_live::inference | Exécute les inférences Parakeet et écrit les WAV de fenêtre. `crates/whisper-adapters/src/parakeet_live/inference.rs:1-531` | 531 | 0 |
| crate::parakeet_live::lifecycle | Démarre, arrête et draine la capture et expose l’import final automatique. `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:1-452` | 430 | 22 |
| crate::parakeet_live::microphone_spectrum_tests | Tests ou supports : définit `parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav` `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:11-216`. | 0 | 216 |
| crate::parakeet_live::recovery_tests | Tests ou supports : définit `unified_capture_recycles_slots_and_preserves_audio_beyond_ring_capacity` `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:4-65`, `unified_capture_pump_converts_synthetic_input_and_closes_the_complete_source` `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:68-96`. | 0 | 139 |
| crate::parakeet_live::trace | Enregistre les traces conditionnelles du micro Parakeet. `crates/whisper-adapters/src/parakeet_live/trace.rs:1-65` | 19 | 46 |
| crate::parakeet_live::window_result_contract_tests | Tests ou supports : définit `correlated_cpu_result_does_not_require_native_call_marker` `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:27-31`, `expected_gpu_result_requires_native_call_marker` `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:34-39`. | 0 | 57 |
| crate::parakeet_live::windows | Planifie et soumet les fenêtres live puis vérifie identité et mots retournés. `crates/whisper-adapters/src/parakeet_live/windows.rs:1-252` | 252 | 0 |
| crate::parakeet_live | Définit l’état et les canaux du micro Parakeet et déclare ses sous-modules. `crates/whisper-adapters/src/parakeet_live.rs:1-163` | 151 | 12 |
| crate::parakeet_pipeline | Prépare et engage les fenêtres finales/previews à partir d’une source PCM. `crates/whisper-adapters/src/parakeet_pipeline.rs:1-1077` | 614 | 463 |
| crate::path_settings | Lit/écrit les préférences JSON et ouvre les sélecteurs Windows. `crates/whisper-adapters/src/path_settings.rs:1-551` | 488 | 63 |
| crate::queue_store | Persiste/rejoue la file et valide les demandes de reprise. `crates/whisper-adapters/src/queue_store.rs:1-805` | 598 | 207 |
| crate::recovery | Scanne archives/file et transforme les statuts pour un choix explicite de reprise. `crates/whisper-adapters/src/recovery.rs:1-45` | 45 | 0 |
| crate::source_binding | Copie/identifie les sources et gère registre, réservations, reprise et retrait des snapshots. `crates/whisper-adapters/src/source_binding.rs:1-1716` | 1519 | 197 |
| crate::staging | Écrit les blocs PCM et checkpoints durables via une file bornée. `crates/whisper-adapters/src/staging.rs:1-552` | 459 | 93 |
| crate::supervisor | Sélectionne les exécutables et suit progression et diagnostics tournants. `crates/whisper-adapters/src/supervisor.rs:1-676` | 462 | 214 |
| crate::vad | Enveloppe WebRTC VAD pour les trames audio de 320 échantillons. `crates/whisper-adapters/src/vad.rs:1-32` | 32 | 0 |
| crate::windows_file_identity | Ouvre les fichiers Windows, vérifie leur identité et les marque pour suppression. `crates/whisper-adapters/src/windows_file_identity.rs:1-314` | 237 | 77 |
| crate::worker_ipc::backends | Lie les demandes au catalogue, qualifie le GPU et suit les workers réutilisables. `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:1-468` | 468 | 0 |
| crate::worker_ipc::backends::native | Résout les routes natives unifiées CPU/GPU et leurs preuves. `crates/whisper-adapters/src/worker_ipc/backends/native.rs:1-215` | 215 | 0 |
| crate::worker_ipc::ipc::framing | Lit, écrit et sérialise les trames JSON bornées. `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:1-106` | 106 | 0 |
| crate::worker_ipc::ipc | Lance le worker et gère les files bornées de commandes/événements. `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:1-398` | 382 | 16 |
| crate::worker_ipc::parakeet_unified::executor | Gère le thread d’inférence unifié, ses commandes et le cycle du processus. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:1-462` | 305 | 157 |
| crate::worker_ipc::parakeet_unified | Définit sources/sessions unifiées et restaure le préfixe documentaire. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:1-208` | 200 | 8 |
| crate::worker_ipc::parakeet_unified::native | Vérifie le PCM et exécute les appels Parakeet corrélés. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:1-465` | 342 | 123 |
| crate::worker_ipc::publication | Arbitre atomiquement activation, annulation et engagement de publication. `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:1-73` | 71 | 2 |
| crate::worker_ipc::publication::tests | Tests ou supports : définit `failed_commit_remains_committing_even_after_stop_rollback` `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:4-17`, `rollback_only_restores_a_cancelled_gate` `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:20-29`. | 0 | 29 |
| crate::worker_ipc::runtime::duplicates | Traite doublons et reconfirmations de publications interrompues. `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:1-423` | 423 | 0 |
| crate::worker_ipc::runtime::effects | Distribue les effets et convertit les erreurs en événements d’échec. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237` | 237 | 0 |
| crate::worker_ipc::runtime::import::worker_start | Démarre les workers Turbo/Parakeet et applique les replis CPU. `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:1-665` | 665 | 0 |
| crate::worker_ipc::runtime::import | Pilote préparation, persistance, publication et admission des imports unifiés. `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1-1438` | 1205 | 233 |
| crate::worker_ipc::runtime::live | Pilote plusieurs chemins live, gain, tests micro et arrêts. `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1-1380` | 1353 | 27 |
| crate::worker_ipc::runtime | Porte ImportRuntime et compose dépendances et registres. `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:1-269` | 171 | 98 |
| crate::worker_ipc::runtime::poll::capture | Pompe capture live, fenêtres et télémétrie. `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:1-254` | 254 | 0 |
| crate::worker_ipc::runtime::poll::parakeet | Draine Parakeet historique, VAD et progression des fenêtres import. `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:1-996` | 996 | 0 |
| crate::worker_ipc::runtime::poll | Draine les workers et route leurs événements avec contrôles d’identité. `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:1-1232` | 1226 | 6 |
| crate::worker_ipc::runtime::poll_unified | Draine sessions/documents unifiés et traite les états terminaux. `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:1-1374` | 1312 | 62 |
| crate::worker_ipc::runtime::queue | Ajoute, retire, prépare et scanne les éléments de la file. `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:1-423` | 423 | 0 |
| crate::worker_ipc::runtime::recovery | Valide et prépare la reprise import et le retrait de remplacement. `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:1-369` | 351 | 18 |
| crate::worker_ipc::runtime::source_binding | Gère bindings source, transfert à la reprise et récupération. `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:1-529` | 523 | 6 |
| crate::worker_ipc::runtime::support | Fournit identités, barrières de recette, historique et adaptation ServiceRuntime. `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:1-456` | 365 | 91 |
| crate::worker_ipc::service::automatic_final | Attache l’import automatique et installe exports et états de manifeste. `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:1-320` | 320 | 0 |
| crate::worker_ipc::service | Implémente le port asynchrone et les boucles de service à files bornées. `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1-812` | 767 | 45 |
| crate::worker_ipc::support | Fournit trace mémoire conditionnelle, conversions SHA-256 et écriture WAV. `crates/whisper-adapters/src/worker_ipc/support.rs:1-94` | 94 | 0 |
| crate::worker_ipc::tests | Tests ou supports : définit `duplicate_confirmation_short_refusals_preserve_runtime_identity` `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:2-50`. | 0 | 50 |
| crate::worker_ipc::tests | Tests ou supports : définit `automatic_final_binding_failure_updates_companion_and_clears_identity` `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:2-59`, `pagination_effects_preserve_request_and_generation_identity_on_error` `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:62-105`. | 0 | 105 |
| crate::worker_ipc::tests | Tests ou supports : définit `controlled_worker_and_invalid_times_leave_no_complete_archive` `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:2-76`, `export_collision_keeps_existing_file_and_does_not_publish_unified_archive` `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:79-160`. | 0 | 2068 |
| crate::worker_ipc::tests | Tests ou supports : définit `native_window_pcm_requires_a_bounded_matching_riff_range` `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:2-37`, `native_window_wav_is_short_pcm16_mono_16khz_riff` `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:40-68`. | 0 | 3980 |
| crate::worker_ipc::tests | Tests ou supports : définit `queue_effect_scan_and_remove_preserve_remaining_fifo_entry` `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:2-74`. | 0 | 74 |
| crate::worker_ipc::tests | Tests ou supports : définit `emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets` `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:31-63`, `reprocessing_latest_requires_known_unique_order_and_all_includes_inactive_present` `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:66-104`. | 0 | 2296 |
| crate::worker_ipc::tests | Tests ou supports : définit `r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication` `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:404-424`, `queued_request_identity_allows_only_compute_choice_and_verified_model_path_binding` `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:427-444`. | 0 | 1401 |
| crate::worker_ipc | Tests ou supports : définit `observe_controlled_child` `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:140-164`. | 0 | 189 |
| crate::worker_ipc::tests | Tests ou supports : définit `existing_source_binding_is_reused_and_failed_acquisition_is_not_inserted` `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:2-61`. | 0 | 61 |
| crate::worker_ipc | Tests ou supports : définit `stopped_ack_waits_for_exit_then_allows_completion` `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:6-16`. | 0 | 17 |
| crate::worker_ipc::tests | Tests ou supports : définit `import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio` `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2-390`, `global_close_waits_for_both_capture_publications_and_refuses_new_admission` `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:393-625`. | 0 | 3034 |
| crate::worker_ipc::tests | Tests ou supports : définit `unified_capture_runtime_rejects_third_before_capture_factory` `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:2-96`, `native_unified_capture_open_failure_leaves_no_session_or_capacity` `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:99-135`. | 0 | 2566 |
| crate::worker_ipc::tests | Tests ou supports : définit `unified_capture_after_direct_stop_uses_its_own_gate_and_publishes_audio` `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:87-364`, `unified_start_and_immediate_stop_keep_the_new_session_identity` `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:367-418`. | 0 | 564 |
| crate::worker_ipc::tests | Tests ou supports : définit `recette_policy_l2a2_requires_the_feature` `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:13-23`. | 0 | 656 |
| crate::worker_ipc::tests | Tests ou supports : définit `worker_command_poll_prioritizes_control_and_distinguishes_idle_and_closed` `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:2-40`. | 0 | 40 |
| crate::worker_ipc::tests | Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`. | 0 | 14 |
| crate::worker_ipc | Réexporte transport/service et rassemble états et modules du runtime IPC. `crates/whisper-adapters/src/worker_ipc.rs:1-140` | 85 | 55 |
| integration::decoder_proxy | Tests ou supports : définit `decoder_proxy_rejects_a_stale_generation_before_returning_audio` `crates/whisper-adapters/tests/decoder_proxy.rs:55-72`, `decoder_proxy_accepts_only_matching_source_job_generation_and_range` `crates/whisper-adapters/tests/decoder_proxy.rs:75-85`. | 0 | 111 |
| integration::durability | Tests ou supports : définit `queue_ack_replays_fifo_and_running_work_recovers_without_launch` `crates/whisper-adapters/tests/durability.rs:30-125`, `resume_validation_rejects_later_fifo_row_without_advancing_any_generation` `crates/whisper-adapters/tests/durability.rs:128-183`. | 0 | 183 |
| integration::gpu_inventory | Tests ou supports : définit `maps_integrated_gpu_as_current_without_cuda_eligibility_inference` `crates/whisper-adapters/tests/gpu_inventory.rs:5-15`, `maps_disabled_present_card_and_preserves_ambiguous_windows_error_codes` `crates/whisper-adapters/tests/gpu_inventory.rs:18-26`. | 0 | 44 |
| integration::gpu_qualification_transport | Tests ou supports : définit `qualification_response_is_typed_and_keeps_per_device_identity` `crates/whisper-adapters/tests/gpu_qualification_transport.rs:4-24`. | 0 | 24 |
| integration::gpu_selection_worker | Tests ou supports : définit `gpu_import_and_live_commands_keep_candidate_identity_and_distinct_ordinals` `crates/whisper-adapters/tests/gpu_selection_worker.rs:23-82`, `real_gpu_import_uses_current_windows_candidate_and_local_model_when_enabled` `crates/whisper-adapters/tests/gpu_selection_worker.rs:86-290`. | 0 | 336 |
| integration::gpu_settings | Tests ou supports : définit `old_json_migrates_additively_and_reopens_with_revision_acknowledgement` `crates/whisper-adapters/tests/gpu_settings.rs:62-100`, `temporary_substitution_uses_q_then_restores_preferred_p` `crates/whisper-adapters/tests/gpu_settings.rs:103-138`. | 0 | 203 |
| integration::import_cpu | Tests ou supports : définit `r11_duplicate_import_prompts_before_queue_and_replace_keeps_all_old_archives` `crates/whisper-adapters/tests/import_cpu.rs:26-345`, `real_wav_runs_through_cpu_worker_and_verified_archive` `crates/whisper-adapters/tests/import_cpu.rs:1290-1409`. | 0 | 2249 |
| integration::import_mp3 | Tests ou supports : définit `r11_mp3_content_identity_is_independent_of_filename_and_destination` `crates/whisper-adapters/tests/import_mp3.rs:12-118`, `source_changed_after_queue_admission_is_refused_without_worker_launch` `crates/whisper-adapters/tests/import_mp3.rs:169-264`. | 0 | 471 |
| integration::legacy_model_requests | Tests ou supports : définit `queued_legacy_turbo_path_is_resolved_in_memory_and_durable_record_is_unchanged` `crates/whisper-adapters/tests/legacy_model_requests.rs:72-96`, `unknown_queued_path_is_refused_with_an_explicit_diagnostic` `crates/whisper-adapters/tests/legacy_model_requests.rs:99-118`. | 0 | 163 |
| integration::live_archive | Tests ou supports : définit `attempt_journal_reserves_monotonically_and_enforces_transitions` `crates/whisper-adapters/tests/live_archive.rs:20-66`, `a_live_group_has_one_archive_writer_at_a_time` `crates/whisper-adapters/tests/live_archive.rs:69-94`. | 0 | 847 |
| integration::live_capture | Tests ou supports : définit `capture_conversion_rejects_unsupported_dimensions_and_uses_bounded_slots` `crates/whisper-adapters/tests/live_capture.rs:8-13`, `bounded_pcm_writer_drains_queued_frames_to_a_verified_checkpoint` `crates/whisper-adapters/tests/live_capture.rs:16-45`. | 0 | 78 |
| integration::microphone_level | Tests ou supports : définit `capture_path_accepts_the_declared_microphone_gain_range_and_bounded_queue` `crates/whisper-adapters/tests/microphone_level.rs:5-12`. | 0 | 12 |
| integration::microphone_probe | Tests ou supports : définit `analyzer_publishes_bounded_psd_and_slow_readers_keep_immutable_snapshots` `crates/whisper-adapters/tests/microphone_probe.rs:60-88`, `invalidation_hides_a_snapshot_even_when_a_reader_still_holds_it` `crates/whisper-adapters/tests/microphone_probe.rs:91-104`. | 0 | 203 |
| integration::microphone_settings | Tests ou supports : définit `old_settings_default_to_windows_microphone_without_losing_paths` `crates/whisper-adapters/tests/microphone_settings.rs:4-11`, `microphone_id_is_kept_in_the_same_settings_snapshot` `crates/whisper-adapters/tests/microphone_settings.rs:14-27`. | 0 | 27 |
| integration::model_catalog | Tests ou supports : définit `closed_catalog_reports_both_pinned_model_files_and_exact_observations` `crates/whisper-adapters/tests/model_catalog.rs:66-88`, `missing_and_corrupt_models_are_reported_without_touching_the_catalog_root` `crates/whisper-adapters/tests/model_catalog.rs:91-121`. | 0 | 231 |
| integration::model_preferences | Tests ou supports : définit `v1_backup_is_created_before_v2_and_is_byte_identical_and_never_replaced` `crates/whisper-adapters/tests/model_preferences.rs:51-122`, `path_write_before_preference_migration_preserves_paths_and_v1_backup` `crates/whisper-adapters/tests/model_preferences.rs:125-163`. | 0 | 313 |
| integration::parakeet_archive | Tests ou supports : définit `import_strategy_defaults_to_legacy_and_round_trips_explicit_versions` `crates/whisper-adapters/tests/parakeet_archive.rs:25-60`, `parakeet_model_identity_prepares_persists_and_publishes_txt_srt` `crates/whisper-adapters/tests/parakeet_archive.rs:267-330`. | 0 | 2096 |
| integration::parakeet_archive_personal_history | Tests ou supports : définit `personal_archives_recognize_previous_workers_without_admitting_them_for_execution` `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:55-198`. | 0 | 198 |
| integration::parakeet_command_wire | Tests ou supports : définit `transcribe_v1_accepts_legacy_and_explicit_gpu_fields` `crates/whisper-adapters/tests/parakeet_command_wire.rs:46-84`, `shutdown_v1_has_the_existing_wire_shape` `crates/whisper-adapters/tests/parakeet_command_wire.rs:87-92`. | 0 | 92 |
| integration::parakeet_control | Tests ou supports : définit `stop_kills_and_waits_for_the_isolated_worker_process` `crates/whisper-adapters/tests/parakeet_control.rs:21-49`, `stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication` `crates/whisper-adapters/tests/parakeet_control.rs:53-161`. | 0 | 940 |
| integration::parakeet_import | Tests ou supports : définit `parakeet_worker_configuration_is_confined_to_the_l2a2_runtime` `crates/whisper-adapters/tests/parakeet_import.rs:24-123`, `gpu_app_fr_en_single_pass` `crates/whisper-adapters/tests/parakeet_import.rs:126-143`. | 0 | 3241 |
| integration::parakeet_long | Tests ou supports : définit `en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy` `crates/whisper-adapters/tests/parakeet_long.rs:612-620`, `fr_synthetic_long_reference_and_vad_use_same_candidate` `crates/whisper-adapters/tests/parakeet_long.rs:624-636`. | 0 | 669 |
| integration::parakeet_runtime_isolation | Tests ou supports : définit `personal_import_is_independent_of_historical_environment` `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:12-14`, `historical_gpu_rejects_invalid_log_root_and_qualification` `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:18-85`. | 0 | 85 |
| integration::parakeet_unified_native | Tests ou supports : définit `build_french_and_english_long_session_fixtures_from_voted_sps_clips` `crates/whisper-adapters/tests/parakeet_unified_native.rs:738-754`, `build_physical_rotation_fixture_over_one_gibibyte` `crates/whisper-adapters/tests/parakeet_unified_native.rs:819-821`. | 0 | 933 |
| integration::publication_lock_contract | Tests ou supports : définit `publication_lock_preserves_contents_and_exclusion_until_drop` `crates/whisper-adapters/tests/publication_lock_contract.rs:9-37`. | 0 | 37 |
| integration::reprocessing_baseline | Tests ou supports : définit `r11_application_baseline` `crates/whisper-adapters/tests/reprocessing_baseline.rs:26-191`. | 0 | 548 |
| integration::reprocessing_cleanup | Tests ou supports : définit `reparse_container_is_rejected_before_any_old_artifact_is_removed` `crates/whisper-adapters/tests/reprocessing_cleanup.rs:504-528`, `replaced_container_identity_is_rejected_and_both_directories_are_preserved` `crates/whisper-adapters/tests/reprocessing_cleanup.rs:531-552`. | 0 | 1598 |
| integration::reprocessing_create | Tests ou supports : définit `create_new_publishes_only_its_internal_pair_and_preserves_existing_treatment` `crates/whisper-adapters/tests/reprocessing_create.rs:201-236`, `application_create_new_processes_the_bound_snapshot_and_preserves_prior_publication` `crates/whisper-adapters/tests/reprocessing_create.rs:239-403`. | 0 | 403 |
| integration::source_binding | Tests ou supports : définit `captures_once_reuses_snapshot_and_retire_releases_capacity` `crates/whisper-adapters/tests/source_binding.rs:69-125`, `rebinding_operation_moves_only_its_owned_snapshot_and_record` `crates/whisper-adapters/tests/source_binding.rs:128-142`. | 0 | 637 |
| integration::support::parakeet_fake_worker | Tests ou supports : définit `main` `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:3-31`. | 0 | 31 |
| crate::parakeet_import::runtime_isolation_tests / integration::parakeet_runtime_isolation / integration::support::parakeet_runtime_isolation | Tests ou supports : définit `run_matrix` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:20-197`. | 0 | 197 |
| integration::vad_contract | Tests ou supports : définit `vad_accepts_only_20ms_mono_16khz_frames` `crates/whisper-adapters/tests/vad_contract.rs:4-8`. | 0 | 8 |
| integration::worker_control | Tests ou supports : définit `cpu_choice_uses_only_cpu_worker_even_when_gpu_exists` `crates/whisper-adapters/tests/worker_control.rs:28-36`, `auto_prefers_gpu_and_uses_cpu_when_gpu_binary_is_absent` `crates/whisper-adapters/tests/worker_control.rs:39-50`. | 0 | 102 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| execution | externe | hors tests / cfg | crates/whisper-adapters/src/archive/cleanup/execution.rs | `crates/whisper-adapters/src/archive/cleanup.rs:5` |
| proofs | externe | hors tests / cfg | crates/whisper-adapters/src/archive/cleanup/proofs.rs | `crates/whisper-adapters/src/archive/cleanup.rs:6` |
| catalog | externe | hors tests / cfg | crates/whisper-adapters/src/archive/publication/catalog.rs | `crates/whisper-adapters/src/archive/publication.rs:14` |
| exports | externe | hors tests / cfg | crates/whisper-adapters/src/archive/publication/exports.rs | `crates/whisper-adapters/src/archive/publication.rs:15` |
| live | externe | hors tests / cfg | crates/whisper-adapters/src/archive/recovery/live.rs | `crates/whisper-adapters/src/archive/recovery.rs:12` |
| campaign | externe | hors tests / cfg | crates/whisper-adapters/src/archive/campaign.rs | `crates/whisper-adapters/src/archive.rs:27` |
| cleanup | externe | hors tests / cfg | crates/whisper-adapters/src/archive/cleanup.rs | `crates/whisper-adapters/src/archive.rs:28` |
| identity | externe | hors tests / cfg | crates/whisper-adapters/src/archive/identity.rs | `crates/whisper-adapters/src/archive.rs:29` |
| import | externe | hors tests / cfg | crates/whisper-adapters/src/archive/import.rs | `crates/whisper-adapters/src/archive.rs:30` |
| journal | externe | hors tests / cfg | crates/whisper-adapters/src/archive/journal.rs | `crates/whisper-adapters/src/archive.rs:31` |
| live | externe | hors tests / cfg | crates/whisper-adapters/src/archive/live.rs | `crates/whisper-adapters/src/archive.rs:32` |
| publication | externe | hors tests / cfg | crates/whisper-adapters/src/archive/publication.rs | `crates/whisper-adapters/src/archive.rs:33` |
| recovery | externe | hors tests / cfg | crates/whisper-adapters/src/archive/recovery.rs | `crates/whisper-adapters/src/archive.rs:34` |
| source | externe | hors tests / cfg | crates/whisper-adapters/src/archive/source.rs | `crates/whisper-adapters/src/archive.rs:35` |
| storage | externe | hors tests / cfg | crates/whisper-adapters/src/archive/storage.rs | `crates/whisper-adapters/src/archive.rs:36` |
| tests | externe | tests | crates/whisper-adapters/src/archive/tests.rs | `crates/whisper-adapters/src/archive.rs:580-581` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/capture.rs:810-1186` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/decoder.rs:104-251` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/direct_live/d2_corpus.rs:450-559` |
| capture | externe | hors tests / cfg | crates/whisper-adapters/src/direct_live/capture.rs | `crates/whisper-adapters/src/direct_live.rs:1` |
| outputs | externe | hors tests / cfg | crates/whisper-adapters/src/direct_live/outputs.rs | `crates/whisper-adapters/src/direct_live.rs:2` |
| wav | externe | hors tests / cfg | crates/whisper-adapters/src/direct_live/wav.rs | `crates/whisper-adapters/src/direct_live.rs:3` |
| worker | externe | hors tests / cfg | crates/whisper-adapters/src/direct_live/worker.rs | `crates/whisper-adapters/src/direct_live.rs:4` |
| d2_corpus | externe | tests | crates/whisper-adapters/src/direct_live/d2_corpus.rs | `crates/whisper-adapters/src/direct_live.rs:6-7` |
| test_support | externe | tests | crates/whisper-adapters/src/direct_live/test_support.rs | `crates/whisper-adapters/src/direct_live.rs:8-9` |
| tests | externe | tests | crates/whisper-adapters/src/direct_live/tests.rs | `crates/whisper-adapters/src/direct_live.rs:10-11` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:140-161` |
| windows_native | externe | hors tests / cfg | crates/whisper-adapters/src/gpu_inventory/windows_native.rs | `crates/whisper-adapters/src/gpu_inventory.rs:13-14` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/gpu_inventory.rs:614-855` |
| archive | externe | hors tests / cfg | crates/whisper-adapters/src/archive.rs | `crates/whisper-adapters/src/lib.rs:1` |
| capture | externe | hors tests / cfg | crates/whisper-adapters/src/capture.rs | `crates/whisper-adapters/src/lib.rs:2` |
| decoder | externe | hors tests / cfg | crates/whisper-adapters/src/decoder.rs | `crates/whisper-adapters/src/lib.rs:3` |
| direct_live | externe | hors tests / cfg | crates/whisper-adapters/src/direct_live.rs | `crates/whisper-adapters/src/lib.rs:4` |
| gpu_inventory | externe | hors tests / cfg | crates/whisper-adapters/src/gpu_inventory.rs | `crates/whisper-adapters/src/lib.rs:5` |
| journal | externe | hors tests / cfg | crates/whisper-adapters/src/journal.rs | `crates/whisper-adapters/src/lib.rs:6` |
| microphone_probe | externe | hors tests / cfg | crates/whisper-adapters/src/microphone_probe.rs | `crates/whisper-adapters/src/lib.rs:7` |
| model_catalog | externe | hors tests / cfg | crates/whisper-adapters/src/model_catalog.rs | `crates/whisper-adapters/src/lib.rs:8` |
| parakeet_audio | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_audio.rs | `crates/whisper-adapters/src/lib.rs:9` |
| parakeet_document | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_document.rs | `crates/whisper-adapters/src/lib.rs:10` |
| parakeet_import | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_import.rs | `crates/whisper-adapters/src/lib.rs:11` |
| parakeet_live | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live.rs | `crates/whisper-adapters/src/lib.rs:12` |
| parakeet_pipeline | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_pipeline.rs | `crates/whisper-adapters/src/lib.rs:13` |
| path_settings | externe | hors tests / cfg | crates/whisper-adapters/src/path_settings.rs | `crates/whisper-adapters/src/lib.rs:14` |
| queue_store | externe | hors tests / cfg | crates/whisper-adapters/src/queue_store.rs | `crates/whisper-adapters/src/lib.rs:15` |
| recovery | externe | hors tests / cfg | crates/whisper-adapters/src/recovery.rs | `crates/whisper-adapters/src/lib.rs:16` |
| source_binding | externe | hors tests / cfg | crates/whisper-adapters/src/source_binding.rs | `crates/whisper-adapters/src/lib.rs:17` |
| staging | externe | hors tests / cfg | crates/whisper-adapters/src/staging.rs | `crates/whisper-adapters/src/lib.rs:18` |
| supervisor | externe | hors tests / cfg | crates/whisper-adapters/src/supervisor.rs | `crates/whisper-adapters/src/lib.rs:19` |
| vad | externe | hors tests / cfg | crates/whisper-adapters/src/vad.rs | `crates/whisper-adapters/src/lib.rs:20` |
| windows_file_identity | externe | hors tests / cfg | crates/whisper-adapters/src/windows_file_identity.rs | `crates/whisper-adapters/src/lib.rs:21-22` |
| worker_ipc | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc.rs | `crates/whisper-adapters/src/lib.rs:23` |
| retry_epoch_regression | externe | tests | crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs | `crates/whisper-adapters/src/microphone_probe.rs:23-24` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/microphone_probe.rs:711-1252` |
| execution_artifact_cache_tests | inline | tests | plage locale | `crates/whisper-adapters/src/model_catalog.rs:456-549` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/parakeet_audio.rs:879-1061` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/parakeet_confirmation.rs:562-1398` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/parakeet_document.rs:737-1066` |
| historical_runtime | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_import/historical_runtime.rs | `crates/whisper-adapters/src/parakeet_import.rs:3` |
| worker_log_mode_tests | inline | tests | plage locale | `crates/whisper-adapters/src/parakeet_import.rs:751-769` |
| legacy_runtime_tests | externe | tests | crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs | `crates/whisper-adapters/src/parakeet_import.rs:771-773` |
| runtime_isolation_tests | externe | tests | crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs | `crates/whisper-adapters/src/parakeet_import.rs:775-776` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/parakeet_import.rs:878-937` |
| confirmation | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_confirmation.rs | `crates/whisper-adapters/src/parakeet_live.rs:6-7` |
| capture | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live/capture.rs | `crates/whisper-adapters/src/parakeet_live.rs:48` |
| events | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live/events.rs | `crates/whisper-adapters/src/parakeet_live.rs:49` |
| inference | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live/inference.rs | `crates/whisper-adapters/src/parakeet_live.rs:50` |
| lifecycle | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live/lifecycle.rs | `crates/whisper-adapters/src/parakeet_live.rs:51` |
| trace | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live/trace.rs | `crates/whisper-adapters/src/parakeet_live.rs:52` |
| windows | externe | hors tests / cfg | crates/whisper-adapters/src/parakeet_live/windows.rs | `crates/whisper-adapters/src/parakeet_live.rs:53` |
| diagnostics_absence_tests | externe | tests | crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs | `crates/whisper-adapters/src/parakeet_live.rs:60-61` |
| microphone_spectrum_tests | externe | tests | crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs | `crates/whisper-adapters/src/parakeet_live.rs:62-63` |
| recovery_tests | externe | tests | crates/whisper-adapters/src/parakeet_live/recovery_tests.rs | `crates/whisper-adapters/src/parakeet_live.rs:64-65` |
| window_result_contract_tests | externe | tests | crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs | `crates/whisper-adapters/src/parakeet_live.rs:66-67` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/parakeet_pipeline.rs:615-1077` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/path_settings.rs:489-551` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/queue_store.rs:561-767` |
| recovery_tests | inline | tests | plage locale | `crates/whisper-adapters/src/source_binding.rs:1520-1716` |
| campaign_tests | inline | tests | plage locale | `crates/whisper-adapters/src/staging.rs:460-552` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/supervisor.rs:463-676` |
| tests | inline | tests | plage locale | `crates/whisper-adapters/src/windows_file_identity.rs:238-314` |
| native | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/backends/native.rs | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:13` |
| framing | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/ipc/framing.rs | `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:1` |
| executor | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs | `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:7` |
| native | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs | `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:8` |
| tests | externe | tests | crates/whisper-adapters/src/worker_ipc/publication/tests.rs | `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:72-73` |
| worker_start | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs | `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:3` |
| duplicates | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:3` |
| effects | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/effects.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:4` |
| import | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/import.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:5` |
| live | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/live.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:6` |
| poll | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/poll.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:7` |
| poll_unified | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:8` |
| queue | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/queue.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:11` |
| recovery | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:12` |
| source_binding | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:13` |
| support | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/support.rs | `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:14` |
| capture | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs | `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:3` |
| parakeet | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs | `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:4` |
| automatic_final | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs | `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1` |
| parakeet_stopped_ack_tests | inline | tests | plage locale | `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17` |
| backends | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/backends/mod.rs | `crates/whisper-adapters/src/worker_ipc.rs:1` |
| ipc | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/ipc/mod.rs | `crates/whisper-adapters/src/worker_ipc.rs:2` |
| parakeet_unified | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs | `crates/whisper-adapters/src/worker_ipc.rs:3` |
| publication | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/publication/mod.rs | `crates/whisper-adapters/src/worker_ipc.rs:4` |
| runtime | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/runtime/mod.rs | `crates/whisper-adapters/src/worker_ipc.rs:5` |
| service | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/service/mod.rs | `crates/whisper-adapters/src/worker_ipc.rs:29` |
| support | externe | hors tests / cfg | crates/whisper-adapters/src/worker_ipc/support.rs | `crates/whisper-adapters/src/worker_ipc.rs:30` |
| tests | externe | tests | crates/whisper-adapters/src/worker_ipc/tests.rs | `crates/whisper-adapters/src/worker_ipc.rs:139-140` |
| r5_2_long_tests | inline | tests | plage locale | `crates/whisper-adapters/tests/parakeet_archive.rs:1444-2096` |
| parakeet_import | inline | tests | plage locale | `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-3` |
| support | inline | tests | plage locale | `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:5-15` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate::archive::campaign — `crates/whisper-adapters/src/archive/campaign.rs`

Contient barrières et marqueurs de campagne conditionnés par recette-hooks. `crates/whisper-adapters/src/archive/campaign.rs:1-484`

Mesure : 484 production / 0 tests / 484 physiques. `crates/whisper-adapters/src/archive/campaign.rs:1-484`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/campaign.rs:1-484`.

Fonctions avec corps : `campaign_after_hook` `crates/whisper-adapters/src/archive/campaign.rs:10-28`; `campaign_is_after_hook` `crates/whisper-adapters/src/archive/campaign.rs:31-49`; `assert_campaign_archive` `crates/whisper-adapters/src/archive/campaign.rs:52-82`; `assert_campaign_archive` `crates/whisper-adapters/src/archive/campaign.rs:85-87`; `write_campaign_marker` `crates/whisper-adapters/src/archive/campaign.rs:90-100`; `campaign_archive_snapshot` `crates/whisper-adapters/src/archive/campaign.rs:103-158`; `record_campaign_operation_result` `crates/whisper-adapters/src/archive/campaign.rs:161-243`; `record_campaign_operation_result` `crates/whisper-adapters/src/archive/campaign.rs:246-255`; `campaign_gate` `crates/whisper-adapters/src/archive/campaign.rs:259-474`; `campaign_gate` `crates/whisper-adapters/src/archive/campaign.rs:477-484`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/campaign.rs:1-484`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/campaign.rs:1-484`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::cleanup::execution — `crates/whisper-adapters/src/archive/cleanup/execution.rs`

Exécute les suppressions de remplacement après contrôle des identités et chemins. `crates/whisper-adapters/src/archive/cleanup/execution.rs:1-561`

Mesure : 561 production / 0 tests / 561 physiques. `crates/whisper-adapters/src/archive/cleanup/execution.rs:1-561`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/cleanup/execution.rs:1-561`.

Fonctions avec corps : `runtime_identity` `crates/whisper-adapters/src/archive/cleanup/execution.rs:16-24`; `run_cleanup_plan_locked` `crates/whisper-adapters/src/archive/cleanup/execution.rs:27-540`; `run_cleanup_plan_locked` `crates/whisper-adapters/src/archive/cleanup/execution.rs:543-548`; `safe_cleanup_path` `crates/whisper-adapters/src/archive/cleanup/execution.rs:550-561`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/cleanup/execution.rs:1-561`.

Dépend de, via `use crate::` : hors tests : `crate::windows_file_identity::VerifiedFile` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::windows_file_identity::delete_if_identity_matches` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::windows_file_identity::open_and_hash_source` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::windows_file_identity` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::cleanup::proofs — `crates/whisper-adapters/src/archive/cleanup/proofs.rs`

Relève les identités des fichiers et conteneurs contrôlés lors du retrait. `crates/whisper-adapters/src/archive/cleanup/proofs.rs:1-569`

Mesure : 569 production / 0 tests / 569 physiques. `crates/whisper-adapters/src/archive/cleanup/proofs.rs:1-569`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/cleanup/proofs.rs:1-569`.

Fonctions avec corps : `capture_cleanup_target` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:19-194`; `attach_empty_container_proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:197-277`; `attach_empty_container_proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:280-287`; `capture_cleanup_ancestor` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:290-321`; `capture_cleanup_file` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:324-350`; `capture_successor_files` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:353-375`; `recover_published_cleanup_successor` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:378-522`; `recover_published_cleanup_successor` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:525-530`; `capture_cleanup_target` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:533-538`; `identity_snapshot` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:541-549`; `identity_snapshot` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:552-556`; `relative_to_root` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:558-569`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/cleanup/proofs.rs:1-569`.

Dépend de, via `use crate::` : hors tests : `crate::windows_file_identity::inspect` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:23`<br>hors tests : `crate::windows_file_identity::inspect` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:203`<br>hors tests : `crate::windows_file_identity::inspect` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:295`<br>hors tests : `crate::windows_file_identity::inspect` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:328`<br>hors tests : `crate::windows_file_identity::VerifiedFile` → `crate::windows_file_identity` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:382`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::cleanup — `crates/whisper-adapters/src/archive/cleanup.rs`

Lit/écrit les plans de retrait des traitements remplacés et permet leur reprise. `crates/whisper-adapters/src/archive/cleanup.rs:1-220`

Mesure : 220 production / 0 tests / 220 physiques. `crates/whisper-adapters/src/archive/cleanup.rs:1-220`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/cleanup.rs:1-220`.

Fonctions avec corps : `replacement_cleanups` `crates/whisper-adapters/src/archive/cleanup.rs:15-60`; `resume_replacement_cleanup` `crates/whisper-adapters/src/archive/cleanup.rs:62-80`; `cleanup_plans_path` `crates/whisper-adapters/src/archive/cleanup.rs:83-86`; `cleanup_plan_path` `crates/whisper-adapters/src/archive/cleanup.rs:88-90`; `read_cleanup_plan` `crates/whisper-adapters/src/archive/cleanup.rs:92-105`; `write_cleanup_plan` `crates/whisper-adapters/src/archive/cleanup.rs:107-123`; `cleanup_remaining` `crates/whisper-adapters/src/archive/cleanup.rs:125-137`; `cleanup_targets_match_intent` `crates/whisper-adapters/src/archive/cleanup.rs:139-179`; `cleanup_container_matches` `crates/whisper-adapters/src/archive/cleanup.rs:181-196`; `cleanup_preserved` `crates/whisper-adapters/src/archive/cleanup.rs:198-220`.

Déclarations `pub` (visibilité sous conditions) : `pub fn replacement_cleanups(root: &Path) -> Result<Vec<ReplacementCleanupInfo>, String> {` `crates/whisper-adapters/src/archive/cleanup.rs:15`; `pub fn resume_replacement_cleanup(root: &Path, cleanup_id: &str) -> Result<(), String> {` `crates/whisper-adapters/src/archive/cleanup.rs:62`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/cleanup.rs:1-220`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::identity — `crates/whisper-adapters/src/archive/identity.rs`

Valide identités/artefacts d’archives et lit les pages document/audio unifiées. `crates/whisper-adapters/src/archive/identity.rs:1-877`

Mesure : 877 production / 0 tests / 877 physiques. `crates/whisper-adapters/src/archive/identity.rs:1-877`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/identity.rs:1-877`.

Fonctions avec corps : `permits_bounded_resume_to` `crates/whisper-adapters/src/archive/identity.rs:10-16`; `parakeet_cpu_test` `crates/whisper-adapters/src/archive/identity.rs:18-41`; `parakeet_gpu_test` `crates/whisper-adapters/src/archive/identity.rs:43-56`; `parakeet_personal` `crates/whisper-adapters/src/archive/identity.rs:58-84`; `is_registered_personal_import` `crates/whisper-adapters/src/archive/identity.rs:86-92`; `is_historical_personal_import` `crates/whisper-adapters/src/archive/identity.rs:94-102`; `is_personal_candidate` `crates/whisper-adapters/src/archive/identity.rs:103-112`; `is_parakeet_import_test` `crates/whisper-adapters/src/archive/identity.rs:114-154`; `is_parakeet_vad_test` `crates/whisper-adapters/src/archive/identity.rs:156-159`; `is_retired_l2a_key` `crates/whisper-adapters/src/archive/identity.rs:161-177`; `is_retired_r5_2_key` `crates/whisper-adapters/src/archive/identity.rs:179-195`; `job_identity_exists` `crates/whisper-adapters/src/archive/identity.rs:199-204`; `read_unified_document_page` `crates/whisper-adapters/src/archive/identity.rs:206-296`; `read_unified_audio_parts_page` `crates/whisper-adapters/src/archive/identity.rs:298-356`; `published_unified_files` `crates/whisper-adapters/src/archive/identity.rs:358-502`; `is_registered_model` `crates/whisper-adapters/src/archive/identity.rs:505-509`; `safe_artifact_path` `crates/whisper-adapters/src/archive/identity.rs:511-516`; `is_sha256` `crates/whisper-adapters/src/archive/identity.rs:518-520`; `parse_job_id` `crates/whisper-adapters/src/archive/identity.rs:522-524`; `verified_artifact` `crates/whisper-adapters/src/archive/identity.rs:526-537`; `verify_artifact` `crates/whisper-adapters/src/archive/identity.rs:538-549`; `verify_internal_unified_artifact` `crates/whisper-adapters/src/archive/identity.rs:551-577`; `canonical_internal_unified_path` `crates/whisper-adapters/src/archive/identity.rs:579-592`; `canonical_generation_directory` `crates/whisper-adapters/src/archive/identity.rs:594-608`; `validate_internal_publication_intent` `crates/whisper-adapters/src/archive/identity.rs:610-694`; `validate_internal_reprocessing_provenance` `crates/whisper-adapters/src/archive/identity.rs:696-756`; `parse_cleanup_origin_generation` `crates/whisper-adapters/src/archive/identity.rs:758-777`; `read_internal_pending` `crates/whisper-adapters/src/archive/identity.rs:779-794`; `verify_pcm_staging` `crates/whisper-adapters/src/archive/identity.rs:796-835`; `canonicalize_for_new_path` `crates/whisper-adapters/src/archive/identity.rs:837-858`; `is_r5_2_vad_recipe_destination` `crates/whisper-adapters/src/archive/identity.rs:860-877`.

Déclarations `pub` (visibilité sous conditions) : `pub fn parakeet_cpu_test() -> Self {` `crates/whisper-adapters/src/archive/identity.rs:18`; `pub fn parakeet_gpu_test() -> Self {` `crates/whisper-adapters/src/archive/identity.rs:43`; `pub fn is_registered_personal_import(&self) -> bool {` `crates/whisper-adapters/src/archive/identity.rs:86`; `pub fn is_parakeet_import_test(&self) -> bool {` `crates/whisper-adapters/src/archive/identity.rs:114`; `pub fn is_parakeet_vad_test(&self) -> bool {` `crates/whisper-adapters/src/archive/identity.rs:156`; `pub fn job_identity_exists(root: &Path, job_id: JobId, generation: Generation) -> bool {` `crates/whisper-adapters/src/archive/identity.rs:199`; `pub fn read_unified_document_page(` `crates/whisper-adapters/src/archive/identity.rs:206`; `pub fn read_unified_audio_parts_page(` `crates/whisper-adapters/src/archive/identity.rs:298`; `pub fn published_unified_files(` `crates/whisper-adapters/src/archive/identity.rs:358`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/identity.rs:1-877`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::import — `crates/whisper-adapters/src/archive/import.rs`

Prépare les générations import, sources segmentées et confirmations VAD. `crates/whisper-adapters/src/archive/import.rs:1-517`

Mesure : 517 production / 0 tests / 517 physiques. `crates/whisper-adapters/src/archive/import.rs:1-517`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/import.rs:1-517`.

Fonctions avec corps : `replace_pending_qualification` `crates/whisper-adapters/src/archive/import.rs:14-50`; `confirm_import_vad_window` `crates/whisper-adapters/src/archive/import.rs:54-118`; `confirmed_import_vad_offset` `crates/whisper-adapters/src/archive/import.rs:120-133`; `prepare` `crates/whisper-adapters/src/archive/import.rs:135-137`; `prepare_with_catalog_path` `crates/whisper-adapters/src/archive/import.rs:141-147`; `prepare_with_qualification` `crates/whisper-adapters/src/archive/import.rs:149-156`; `prepare_segmented_capture` `crates/whisper-adapters/src/archive/import.rs:160-199`; `prepare_with_source_binding` `crates/whisper-adapters/src/archive/import.rs:201-217`; `prepare_inner` `crates/whisper-adapters/src/archive/import.rs:219-231`; `prepare_inner_with_publication_lock` `crates/whisper-adapters/src/archive/import.rs:233-249`; `prepare_inner_impl` `crates/whisper-adapters/src/archive/import.rs:251-516`.

Déclarations `pub` (visibilité sous conditions) : `pub fn replace_pending_qualification(` `crates/whisper-adapters/src/archive/import.rs:14`; `pub fn confirm_import_vad_window(` `crates/whisper-adapters/src/archive/import.rs:54`; `pub fn confirmed_import_vad_offset(&self) -> Result<u64, String> {` `crates/whisper-adapters/src/archive/import.rs:120`; `pub fn prepare(&mut self, request: ImportRequest) -> Result<ImportRequest, String> {` `crates/whisper-adapters/src/archive/import.rs:135`; `pub fn prepare_with_catalog_path(` `crates/whisper-adapters/src/archive/import.rs:141`; `pub fn prepare_with_qualification(` `crates/whisper-adapters/src/archive/import.rs:149`; `pub fn prepare_segmented_capture(` `crates/whisper-adapters/src/archive/import.rs:160`; `pub fn prepare_with_source_binding(` `crates/whisper-adapters/src/archive/import.rs:201`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/import.rs:1-517`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::journal — `crates/whisper-adapters/src/archive/journal.rs`

Persiste et relit les segments et valide le préfixe de reprise. `crates/whisper-adapters/src/archive/journal.rs:1-308`

Mesure : 308 production / 0 tests / 308 physiques. `crates/whisper-adapters/src/archive/journal.rs:1-308`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/journal.rs:1-308`.

Fonctions avec corps : `persist_segment` `crates/whisper-adapters/src/archive/journal.rs:7-39`; `read_segments` `crates/whisper-adapters/src/archive/journal.rs:42-61`; `validate_segment_journal` `crates/whisper-adapters/src/archive/journal.rs:63-77`; `visit_segments` `crates/whisper-adapters/src/archive/journal.rs:79-105`; `visit_segments_read_only` `crates/whisper-adapters/src/archive/journal.rs:107-127`; `validate_unified_resume_prefix` `crates/whisper-adapters/src/archive/journal.rs:129-234`; `read_segment_journal_line` `crates/whisper-adapters/src/archive/journal.rs:236-270`; `repair_uncommitted_segment_tail` `crates/whisper-adapters/src/archive/journal.rs:272-308`.

Déclarations `pub` (visibilité sous conditions) : `pub fn persist_segment(&mut self, segment: &WorkerSegment) -> Result<u64, String> {` `crates/whisper-adapters/src/archive/journal.rs:7`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/journal.rs:1-308`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::live — `crates/whisper-adapters/src/archive/live.rs`

Persiste paquets MP3, tentatives et fenêtres live puis publie les artefacts. `crates/whisper-adapters/src/archive/live.rs:1-714`

Mesure : 714 production / 0 tests / 714 physiques. `crates/whisper-adapters/src/archive/live.rs:1-714`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/live.rs:1-714`.

Fonctions avec corps : `prepare` `crates/whisper-adapters/src/archive/live.rs:12-135`; `capture_started` `crates/whisper-adapters/src/archive/live.rs:138-152`; `append_mp3_packet` `crates/whisper-adapters/src/archive/live.rs:154-160`; `reserve_attempt` `crates/whisper-adapters/src/archive/live.rs:162-173`; `mark_attempt_started` `crates/whisper-adapters/src/archive/live.rs:175-182`; `retire_attempt` `crates/whisper-adapters/src/archive/live.rs:184-191`; `restart_mp3_attempt` `crates/whisper-adapters/src/archive/live.rs:193-203`; `discard_unconfirmed_segments` `crates/whisper-adapters/src/archive/live.rs:205-222`; `append_attempt_record` `crates/whisper-adapters/src/archive/live.rs:224-286`; `persist_segment` `crates/whisper-adapters/src/archive/live.rs:288-339`; `persist_window_finished` `crates/whisper-adapters/src/archive/live.rs:341-442`; `confirmed_samples` `crates/whisper-adapters/src/archive/live.rs:444-446`; `next_segment_sequence` `crates/whisper-adapters/src/archive/live.rs:448-450`; `next_window_sequence` `crates/whisper-adapters/src/archive/live.rs:452-454`; `coverage_path` `crates/whisper-adapters/src/archive/live.rs:456-461`; `publish` `crates/whisper-adapters/src/archive/live.rs:463-642`; `resolve_live_publication` `crates/whisper-adapters/src/archive/live.rs:648-713`.

Déclarations `pub` (visibilité sous conditions) : `pub fn prepare(root: impl AsRef<Path>, mut request: LiveRequest) -> Result<Self, String> {` `crates/whisper-adapters/src/archive/live.rs:12`; `pub fn capture_started(&mut self) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:138`; `pub fn append_mp3_packet(&mut self, packet: &[u8]) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:154`; `pub fn reserve_attempt(&mut self) -> Result<u64, String> {` `crates/whisper-adapters/src/archive/live.rs:162`; `pub fn mark_attempt_started(&mut self, attempt: u64) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:175`; `pub fn retire_attempt(&mut self, attempt: u64) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:184`; `pub fn restart_mp3_attempt(&mut self) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:193`; `pub fn discard_unconfirmed_segments(&mut self) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:205`; `pub fn persist_segment(` `crates/whisper-adapters/src/archive/live.rs:288`; `pub fn persist_window_finished(` `crates/whisper-adapters/src/archive/live.rs:341`; `pub fn confirmed_samples(&self) -> u64 {` `crates/whisper-adapters/src/archive/live.rs:444`; `pub fn next_segment_sequence(&self) -> u64 {` `crates/whisper-adapters/src/archive/live.rs:448`; `pub fn next_window_sequence(&self) -> u64 {` `crates/whisper-adapters/src/archive/live.rs:452`; `pub fn publish(` `crates/whisper-adapters/src/archive/live.rs:463`; `pub fn resolve_live_publication(root: &Path, request: &LiveRequest) -> Result<(), String> {` `crates/whisper-adapters/src/archive/live.rs:648`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/live.rs:1-714`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::publication::catalog — `crates/whisper-adapters/src/archive/publication/catalog.rs`

Gère index, réservations, doublons et intentions de publication. `crates/whisper-adapters/src/archive/publication/catalog.rs:1-1082`

Mesure : 1082 production / 0 tests / 1082 physiques. `crates/whisper-adapters/src/archive/publication/catalog.rs:1-1082`

Types : `PublicationReservation` (struct, hors tests) `crates/whisper-adapters/src/archive/publication/catalog.rs:718-725`.

Fonctions avec corps : `corrupt_entry_is_foreign` `crates/whisper-adapters/src/archive/publication/catalog.rs:13-85`; `lock_publications` `crates/whisper-adapters/src/archive/publication/catalog.rs:87-101`; `published_imports` `crates/whisper-adapters/src/archive/publication/catalog.rs:103-112`; `published_imports_for_strategy` `crates/whisper-adapters/src/archive/publication/catalog.rs:114-154`; `reprocessing_catalog` `crates/whisper-adapters/src/archive/publication/catalog.rs:159-214`; `reserve_new_import` `crates/whisper-adapters/src/archive/publication/catalog.rs:216-231`; `reserve_create_only_import` `crates/whisper-adapters/src/archive/publication/catalog.rs:235-253`; `reserve_create_only_new_import` `crates/whisper-adapters/src/archive/publication/catalog.rs:257-274`; `reserve_duplicate_import` `crates/whisper-adapters/src/archive/publication/catalog.rs:276-311`; `reserve_reprocessing_import` `crates/whisper-adapters/src/archive/publication/catalog.rs:315-405`; `suggest_duplicate_export_basename` `crates/whisper-adapters/src/archive/publication/catalog.rs:407-412`; `publication_reconfirmation_required` `crates/whisper-adapters/src/archive/publication/catalog.rs:417-424`; `reconfirm_interrupted_publication` `crates/whisper-adapters/src/archive/publication/catalog.rs:428-541`; `clear_duplicate_reservation` `crates/whisper-adapters/src/archive/publication/catalog.rs:543-556`; `reconcile_publications` `crates/whisper-adapters/src/archive/publication/catalog.rs:558-565`; `resolve_import_publication` `crates/whisper-adapters/src/archive/publication/catalog.rs:567-611`; `publication_index_path` `crates/whisper-adapters/src/archive/publication/catalog.rs:614-616`; `publication_reservation_path` `crates/whisper-adapters/src/archive/publication/catalog.rs:618-626`; `publication_reconfirmation_required_unlocked` `crates/whisper-adapters/src/archive/publication/catalog.rs:628-693`; `write_publication_reservation` `crates/whisper-adapters/src/archive/publication/catalog.rs:695-716`; `write_publication_reservation_with_storage` `crates/whisper-adapters/src/archive/publication/catalog.rs:727-760`; `write_publication_intent` `crates/whisper-adapters/src/archive/publication/catalog.rs:762-787`; `load_publication_intent` `crates/whisper-adapters/src/archive/publication/catalog.rs:789-835`; `read_publication_index` `crates/whisper-adapters/src/archive/publication/catalog.rs:837-847`; `write_publication_index` `crates/whisper-adapters/src/archive/publication/catalog.rs:849-860`; `apply_publication_intent` `crates/whisper-adapters/src/archive/publication/catalog.rs:862-909`; `reserve_export_basename` `crates/whisper-adapters/src/archive/publication/catalog.rs:911-959`; `safe_export_stem` `crates/whisper-adapters/src/archive/publication/catalog.rs:961-978`; `reconcile_replaced_exports` `crates/whisper-adapters/src/archive/publication/catalog.rs:980-1082`.

Déclarations `pub` (visibilité sous conditions) : `pub fn lock_publications(root: &Path) -> Result<PublicationLock, String> {` `crates/whisper-adapters/src/archive/publication/catalog.rs:87`; `pub fn published_imports(` `crates/whisper-adapters/src/archive/publication/catalog.rs:103`; `pub fn published_imports_for_strategy(` `crates/whisper-adapters/src/archive/publication/catalog.rs:114`; `pub fn reprocessing_catalog(` `crates/whisper-adapters/src/archive/publication/catalog.rs:159`; `pub fn reserve_new_import(` `crates/whisper-adapters/src/archive/publication/catalog.rs:216`; `pub fn reserve_create_only_import(` `crates/whisper-adapters/src/archive/publication/catalog.rs:235`; `pub fn reserve_create_only_new_import(` `crates/whisper-adapters/src/archive/publication/catalog.rs:257`; `pub fn reserve_duplicate_import(` `crates/whisper-adapters/src/archive/publication/catalog.rs:276`; `pub fn reserve_reprocessing_import(` `crates/whisper-adapters/src/archive/publication/catalog.rs:315`; `pub fn suggest_duplicate_export_basename(` `crates/whisper-adapters/src/archive/publication/catalog.rs:407`; `pub fn publication_reconfirmation_required(` `crates/whisper-adapters/src/archive/publication/catalog.rs:417`; `pub fn reconfirm_interrupted_publication(` `crates/whisper-adapters/src/archive/publication/catalog.rs:428`; `pub fn clear_duplicate_reservation(` `crates/whisper-adapters/src/archive/publication/catalog.rs:543`; `pub fn reconcile_publications(root: &Path) -> Result<(), String> {` `crates/whisper-adapters/src/archive/publication/catalog.rs:558`; `pub fn resolve_import_publication(` `crates/whisper-adapters/src/archive/publication/catalog.rs:567`.

Dépend de, via `use crate::` : hors tests : `crate::archive::PublicationReconfirmation` → `crate::archive` `crates/whisper-adapters/src/archive/publication/catalog.rs:11`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::publication::exports — `crates/whisper-adapters/src/archive/publication/exports.rs`

Installe les exports et vérifie leur propriété et identité de fichier. `crates/whisper-adapters/src/archive/publication/exports.rs:1-623`

Mesure : 615 production / 8 tests / 623 physiques. `crates/whisper-adapters/src/archive/publication/exports.rs:1-623`

Types : `FileIdentityHasher` (struct, hors tests) `crates/whisper-adapters/src/archive/publication/exports.rs:266-268`; `ExportInstallOps` (struct, hors tests) `crates/whisper-adapters/src/archive/publication/exports.rs:329-332`.

Fonctions avec corps : `install_publication_exports` `crates/whisper-adapters/src/archive/publication/exports.rs:10-18`; `install_publication_exports_with` `crates/whisper-adapters/src/archive/publication/exports.rs:20-48`; `install_publication_exports_with_copy_hook` `crates/whisper-adapters/src/archive/publication/exports.rs:50-236`; `same_export_ownership` `crates/whisper-adapters/src/archive/publication/exports.rs:238-248`; `export_install_temporary_path` `crates/whisper-adapters/src/archive/publication/exports.rs:250-255`; `persisted_file_identity` `crates/whisper-adapters/src/archive/publication/exports.rs:257-263`; `finish` `crates/whisper-adapters/src/archive/publication/exports.rs:271-273`; `write` `crates/whisper-adapters/src/archive/publication/exports.rs:275-277`; `write_u8` `crates/whisper-adapters/src/archive/publication/exports.rs:279-281`; `write_u64` `crates/whisper-adapters/src/archive/publication/exports.rs:283-285`; `write_usize` `crates/whisper-adapters/src/archive/publication/exports.rs:287-289`; `set_export_file_identity` `crates/whisper-adapters/src/archive/publication/exports.rs:292-302`; `set_export_temporary_identity` `crates/whisper-adapters/src/archive/publication/exports.rs:304-314`; `copy_file_create_new` `crates/whisper-adapters/src/archive/publication/exports.rs:316-327`; `install_owned_export` `crates/whisper-adapters/src/archive/publication/exports.rs:334-412`; `intent_job_id` `crates/whisper-adapters/src/archive/publication/exports.rs:414-417`; `intent_generation` `crates/whisper-adapters/src/archive/publication/exports.rs:419-422`; `parse_generation_directory` `crates/whisper-adapters/src/archive/publication/exports.rs:424-440`; `verify_owned_export` `crates/whisper-adapters/src/archive/publication/exports.rs:442-519`; `is_archived_export_file` `crates/whisper-adapters/src/archive/publication/exports.rs:521-623`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/publication/exports.rs:1-623`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/publication/exports.rs:1-623`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::publication — `crates/whisper-adapters/src/archive/publication.rs`

Prépare puis engage la publication d’une génération import. `crates/whisper-adapters/src/archive/publication.rs:1-291`

Mesure : 287 production / 4 tests / 291 physiques. `crates/whisper-adapters/src/archive/publication.rs:1-291`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/publication.rs:1-291`.

Fonctions avec corps : `prepare_publish` `crates/whisper-adapters/src/archive/publication.rs:31-133`; `commit_publish` `crates/whisper-adapters/src/archive/publication.rs:135-280`; `publish` `crates/whisper-adapters/src/archive/publication.rs:282-290`.

Déclarations `pub` (visibilité sous conditions) : `pub fn prepare_publish(` `crates/whisper-adapters/src/archive/publication.rs:31`; `pub fn commit_publish(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {` `crates/whisper-adapters/src/archive/publication.rs:135`; `pub fn publish(` `crates/whisper-adapters/src/archive/publication.rs:282`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/publication.rs:1-291`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::recovery::live — `crates/whisper-adapters/src/archive/recovery/live.rs`

Scanne générations live/préfixes confirmés et réconcilie les alias. `crates/whisper-adapters/src/archive/recovery/live.rs:1-405`

Mesure : 405 production / 0 tests / 405 physiques. `crates/whisper-adapters/src/archive/recovery/live.rs:1-405`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/recovery/live.rs:1-405`.

Fonctions avec corps : `scan_live` `crates/whisper-adapters/src/archive/recovery/live.rs:9-101`; `reconcile_alias` `crates/whisper-adapters/src/archive/recovery/live.rs:103-114`; `live_current_generation` `crates/whisper-adapters/src/archive/recovery/live.rs:116-145`; `read_verified_live_manifest` `crates/whisper-adapters/src/archive/recovery/live.rs:147-183`; `scan_live_recoveries` `crates/whisper-adapters/src/archive/recovery/live.rs:185-293`; `scan_confirmed_live_prefix` `crates/whisper-adapters/src/archive/recovery/live.rs:295-362`; `read_bounded_jsonl` `crates/whisper-adapters/src/archive/recovery/live.rs:364-405`.

Déclarations `pub` (visibilité sous conditions) : `pub fn scan_live_recoveries(` `crates/whisper-adapters/src/archive/recovery/live.rs:185`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/recovery/live.rs:1-405`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::recovery — `crates/whisper-adapters/src/archive/recovery.rs`

Scanne les générations et prépare la reprise depuis un préfixe vérifié. `crates/whisper-adapters/src/archive/recovery.rs:1-752`

Mesure : 752 production / 0 tests / 752 physiques. `crates/whisper-adapters/src/archive/recovery.rs:1-752`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/recovery.rs:1-752`.

Fonctions avec corps : `resume_from` `crates/whisper-adapters/src/archive/recovery.rs:21-27`; `resume_from_with_catalog_path` `crates/whisper-adapters/src/archive/recovery.rs:30-37`; `resume_from_with_qualification` `crates/whisper-adapters/src/archive/recovery.rs:39-312`; `copy_resume_prefix` `crates/whisper-adapters/src/archive/recovery.rs:314-343`; `copy_unified_resume_prefix` `crates/whisper-adapters/src/archive/recovery.rs:345-378`; `scan` `crates/whisper-adapters/src/archive/recovery.rs:380-383`; `scan_publications` `crates/whisper-adapters/src/archive/recovery.rs:385-428`; `scan_raw` `crates/whisper-adapters/src/archive/recovery.rs:430-499`; `scan_generation` `crates/whisper-adapters/src/archive/recovery.rs:502-603`; `validate_pending` `crates/whisper-adapters/src/archive/recovery.rs:605-652`; `read_import_vad_windows` `crates/whisper-adapters/src/archive/recovery.rs:654-716`; `initialize_pending` `crates/whisper-adapters/src/archive/recovery.rs:718-739`; `corrupt_entry` `crates/whisper-adapters/src/archive/recovery.rs:741-752`.

Déclarations `pub` (visibilité sous conditions) : `pub use live::scan_live_recoveries;` `crates/whisper-adapters/src/archive/recovery.rs:14`; `pub fn resume_from(` `crates/whisper-adapters/src/archive/recovery.rs:21`; `pub fn resume_from_with_catalog_path(` `crates/whisper-adapters/src/archive/recovery.rs:30`; `pub fn resume_from_with_qualification(` `crates/whisper-adapters/src/archive/recovery.rs:39`; `pub fn scan(root: &Path) -> Result<Vec<ArchiveEntry>, String> {` `crates/whisper-adapters/src/archive/recovery.rs:380`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/recovery.rs:1-752`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::source — `crates/whisper-adapters/src/archive/source.rs`

Identifie et empreinte les sources simples/segmentées et mesure leur durée. `crates/whisper-adapters/src/archive/source.rs:1-198`

Mesure : 198 production / 0 tests / 198 physiques. `crates/whisper-adapters/src/archive/source.rs:1-198`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/source.rs:1-198`.

Fonctions avec corps : `identify_source` `crates/whisper-adapters/src/archive/source.rs:13-26`; `segmented_pcm_manifest` `crates/whisper-adapters/src/archive/source.rs:29-112`; `sha256_file` `crates/whisper-adapters/src/archive/source.rs:114-127`; `source_output_samples` `crates/whisper-adapters/src/archive/source.rs:129-132`; `source_output_samples_inner` `crates/whisper-adapters/src/archive/source.rs:134-194`; `hex` `crates/whisper-adapters/src/archive/source.rs:196-198`.

Déclarations `pub` (visibilité sous conditions) : `pub fn identify_source(request: &mut ImportRequest) -> Result<(), String> {` `crates/whisper-adapters/src/archive/source.rs:13`; `pub fn sha256_file(path: &Path) -> Result<[u8; 32], String> {` `crates/whisper-adapters/src/archive/source.rs:114`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/source.rs:1-198`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::storage — `crates/whisper-adapters/src/archive/storage.rs`

Écrit, copie et synchronise des fichiers nouveaux ou remplacés. `crates/whisper-adapters/src/archive/storage.rs:1-66`

Mesure : 66 production / 0 tests / 66 physiques. `crates/whisper-adapters/src/archive/storage.rs:1-66`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/storage.rs:1-66`.

Fonctions avec corps : `write_replace_streamed` `crates/whisper-adapters/src/archive/storage.rs:3-31`; `copy_replace_streamed` `crates/whisper-adapters/src/archive/storage.rs:33-39`; `write_new_synced` `crates/whisper-adapters/src/archive/storage.rs:41-49`; `write_replace_synced` `crates/whisper-adapters/src/archive/storage.rs:50-66`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/storage.rs:1-66`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/storage.rs:1-66`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs`

Tests ou supports : définit `recette_policy_l04_requires_the_feature` `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:6-14`, `bounded_resume_accepts_a_current_backend_for_the_same_model_profile` `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:17-27`.

Mesure : 0 production / 715 tests / 715 physiques. `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:1-715`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:1-715`.

Fonctions avec corps : `recette_policy_l04_requires_the_feature` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:6-14`; `bounded_resume_accepts_a_current_backend_for_the_same_model_profile` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:17-27`; `create_only_publication_returns_verified_internal_unified_artifacts_without_external_exports` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:30-249`; `reprocessing_reservation_tracks_only_current_strategy_publications_for_resume` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:253-470`; `canonical_generation_must_be_contained_by_archive_root` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:473-508`; `unified_archive_pages_cap_text_and_audio_entries_at_128_with_real_cursors` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:511-622`; `pending_archive_versions_bind_strategy_and_keep_legacy_default` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:625-664`; `legacy_cleanup_target_without_container_proof_deserializes_without_parent_authority` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:667-688`; `campaign_snapshot_skips_an_active_writer_lock` [tests] `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:692-715`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:1-715`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:1-715`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/campaign.rs`

Tests ou supports : définit `l04_campaign_gate_rejects_escape_and_duplicate_trigger` `crates/whisper-adapters/src/archive/tests/campaign.rs:3-146`.

Mesure : 0 production / 146 tests / 146 physiques. `crates/whisper-adapters/src/archive/tests/campaign.rs:1-146`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/campaign.rs:1-146`.

Fonctions avec corps : `l04_campaign_gate_rejects_escape_and_duplicate_trigger` [tests] `crates/whisper-adapters/src/archive/tests/campaign.rs:3-146`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/campaign.rs:1-146`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/campaign.rs:1-146`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs`

Tests ou supports : définit `archive_characterization_truncated_cleanup_plan_is_refused_without_disposition` `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:2-26`.

Mesure : 0 production / 26 tests / 26 physiques. `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:1-26`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:1-26`.

Fonctions avec corps : `archive_characterization_truncated_cleanup_plan_is_refused_without_disposition` [tests] `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:2-26`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:1-26`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/cleanup_characterization.rs:1-26`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs`

Tests ou supports : définit `archive_characterization_ownership_versions_one_two_three_follow_existing_paths` `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:14-55`, `archive_characterization_ownership_truncated_or_unknown_refuses_without_mutation` `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:58-94`.

Mesure : 0 production / 262 tests / 262 physiques. `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:1-262`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:1-262`.

Fonctions avec corps : `archive_characterization_installed_exports` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:2-11`; `archive_characterization_ownership_versions_one_two_three_follow_existing_paths` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:14-55`; `archive_characterization_ownership_truncated_or_unknown_refuses_without_mutation` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:58-94`; `archive_characterization_v3_final_link_before_identity_is_recovered` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:97-120`; `archive_characterization_reconcile_partial_skips_byte_identical_third_party_then_retries` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:123-187`; `archive_characterization_residual_reservation_cleanup_refusal_and_retry` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:190-233`; `archive_characterization_second_external_resolution_preserves_committed_publication` [tests] `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:236-262`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:1-262`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/export_ownership_characterization.rs:1-262`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/publication_characterization.rs`

Tests ou supports : définit `archive_characterization_stream_error_keeps_final_and_retry_replaces_it` `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:78-100`, `archive_characterization_replace_obstacle_keeps_temporary_until_retry` `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:103-126`.

Mesure : 0 production / 335 tests / 335 physiques. `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:1-335`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:1-335`.

Fonctions avec corps : `archive_characterization_root` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:2-16`; `archive_characterization_pending` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:18-75`; `archive_characterization_stream_error_keeps_final_and_retry_replaces_it` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:78-100`; `archive_characterization_replace_obstacle_keeps_temporary_until_retry` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:103-126`; `archive_characterization_txt_only_prepare_is_retried_before_publication` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:129-154`; `archive_characterization_missing_truncated_and_divergent_manifest_are_refused` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:157-184`; `archive_characterization_intent_and_partial_staging_stay_unpublished_until_retry` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:187-222`; `archive_characterization_divergent_staged_srt_refused_and_retry_succeeds` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:225-252`; `archive_characterization_index_rename_obstacle_preserves_temporary_and_retry` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:255-274`; `archive_characterization_current_before_index_internal_resolution_is_repeatable` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:277-300`; `archive_characterization_index_absent_truncated_and_temporary_retry` [tests] `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:303-335`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:1-335`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:1-335`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive — `crates/whisper-adapters/src/archive/tests/publication_support.rs`

Tests ou supports : définit `inject_copy_fallback_publication_cut` `crates/whisper-adapters/src/archive/tests/publication_support.rs:20-25`, `clear_copy_fallback_publication_cut` `crates/whisper-adapters/src/archive/tests/publication_support.rs:28-32`.

Mesure : 0 production / 59 tests / 59 physiques. `crates/whisper-adapters/src/archive/tests/publication_support.rs:1-59`

Types : `PublicationCopyFault` (struct, tests) `crates/whisper-adapters/src/archive/tests/publication_support.rs:10-13`.

Fonctions avec corps : `inject_copy_fallback_publication_cut` [tests] `crates/whisper-adapters/src/archive/tests/publication_support.rs:20-25`; `clear_copy_fallback_publication_cut` [tests] `crates/whisper-adapters/src/archive/tests/publication_support.rs:28-32`; `publication_copy_fault_applies` [tests] `crates/whisper-adapters/src/archive/tests/publication_support.rs:35-40`; `publication_copy_fault_after` [tests] `crates/whisper-adapters/src/archive/tests/publication_support.rs:43-59`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/publication_support.rs:1-59`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/publication_support.rs:1-59`.

Utilisé par, via ces imports : hors tests : `crate::archive::publication::catalog` `crates/whisper-adapters/src/archive/publication/catalog.rs:11`<br>hors tests : `crate::journal` `crates/whisper-adapters/src/journal.rs:3`<br>hors tests : `crate::recovery` `crates/whisper-adapters/src/recovery.rs:1`<br>hors tests : `crate::recovery` `crates/whisper-adapters/src/recovery.rs:1`<br>hors tests : `crate::recovery` `crates/whisper-adapters/src/recovery.rs:1`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`.

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/recovery.rs`

Tests ou supports : définit `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` `crates/whisper-adapters/src/archive/tests/recovery.rs:2-327`, `segment_recovery_keeps_synced_prefix_and_drops_only_a_partial_tail` `crates/whisper-adapters/src/archive/tests/recovery.rs:330-351`.

Mesure : 0 production / 847 tests / 847 physiques. `crates/whisper-adapters/src/archive/tests/recovery.rs:1-847`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/recovery.rs:1-847`.

Fonctions avec corps : `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` [tests] `crates/whisper-adapters/src/archive/tests/recovery.rs:2-327`; `segment_recovery_keeps_synced_prefix_and_drops_only_a_partial_tail` [tests] `crates/whisper-adapters/src/archive/tests/recovery.rs:330-351`; `scan_preserves_healthy_entries_and_reports_each_corrupt_generation` [tests] `crates/whisper-adapters/src/archive/tests/recovery.rs:354-538`; `published_running_row_is_removed_durably_and_recovery_scans_are_stable` [tests] `crates/whisper-adapters/src/archive/tests/recovery.rs:541-650`; `resume_rejects_canonical_cross_destination_without_mutating_either_root` [tests] `crates/whisper-adapters/src/archive/tests/recovery.rs:653-721`; `resumed_prefix_copy_retries_after_pending_creation_without_duplication` [tests] `crates/whisper-adapters/src/archive/tests/recovery.rs:724-847`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/recovery.rs:1-847`.

Dépend de, via `use crate::` : tests : `crate::queue_store::QueueStatus` → `crate::queue_store` `crates/whisper-adapters/src/archive/tests/recovery.rs:542`<br>tests : `crate::queue_store::QueueStore` → `crate::queue_store` `crates/whisper-adapters/src/archive/tests/recovery.rs:542`<br>tests : `crate::queue_store` → `crate::queue_store` `crates/whisper-adapters/src/archive/tests/recovery.rs:542`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs`

Tests ou supports : définit `archive_characterization_pending_absent_truncated_and_divergent` `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:2-40`, `archive_characterization_segment_journal_repairs_initial_and_large_partial_tail` `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:43-64`.

Mesure : 0 production / 85 tests / 85 physiques. `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:1-85`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:1-85`.

Fonctions avec corps : `archive_characterization_pending_absent_truncated_and_divergent` [tests] `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:2-40`; `archive_characterization_segment_journal_repairs_initial_and_large_partial_tail` [tests] `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:43-64`; `archive_characterization_complete_corrupt_middle_segment_is_not_repaired` [tests] `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:67-85`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:1-85`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/recovery_characterization.rs:1-85`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs`

Tests ou supports : définit `r11_replace_deactivates_every_matching_publication_and_keeps_other_audio` `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:2-87`, `r11_create_new_preserves_existing_publications_and_index_write_is_atomic` `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:90-135`.

Mesure : 0 production / 413 tests / 413 physiques. `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:1-413`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:1-413`.

Fonctions avec corps : `r11_replace_deactivates_every_matching_publication_and_keeps_other_audio` [tests] `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:2-87`; `r11_create_new_preserves_existing_publications_and_index_write_is_atomic` [tests] `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:90-135`; `r11_export_names_remain_reserved_by_the_index_after_visible_files_are_removed` [tests] `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:138-169`; `r11_replaced_export_reconciliation_never_moves_a_third_party_same_name_file` [tests] `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:172-234`; `r11_recovery_between_current_and_index_keeps_legacy_visible_until_explicit_activation` [tests] `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:237-372`; `segmented_capture_identity_references_every_rotated_pcm_part` [tests] `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:375-413`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:1-413`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests/replacement_and_segmented.rs:1-413`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive::tests — `crates/whisper-adapters/src/archive/tests.rs`

Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-adapters/src/archive/tests.rs:1-8`.

Mesure : 0 production / 8 tests / 8 physiques. `crates/whisper-adapters/src/archive/tests.rs:1-8`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/archive/tests.rs:1-8`.

Fonctions avec corps : aucune relevée dans `crates/whisper-adapters/src/archive/tests.rs:1-8`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/archive/tests.rs:1-8`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive/tests.rs:1-8`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::archive — `crates/whisper-adapters/src/archive.rs`

Définit ArchiveStore/LiveArchive et distribue leurs implémentations dans les sous-modules. `crates/whisper-adapters/src/archive.rs:1-581`

Mesure : 566 production / 15 tests / 581 physiques. `crates/whisper-adapters/src/archive.rs:1-581`

Types : `PublicationReconfirmation` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:18-25`; `SegmentRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:95-99`; `PendingRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:102-116`; `ImportVadWindowPayload` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:123-134`; `ImportVadWindowRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:137-140`; `ArchiveQualificationKey` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:152-169`; `ArtifactRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:172-176`; `CompleteRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:179-187`; `InstalledExportOwnership` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:190-206`; `ArchiveEntry` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:213-222`; `PublicationIndex` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:225-228`; `PublicationRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:231-239`; `PublicationIdentity` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:242-245`; `PublicationIntent` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:248-262`; `CleanupObjectIdentity` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:265-269`; `CleanupObject` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:272-280`; `CleanupAncestor` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:287-290`; `CleanupTarget` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:293-300`; `CleanupPlan` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:303-318`; `PublicationLock` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:337-339`; `ExportHardLinker` (type, hors tests) `crates/whisper-adapters/src/archive.rs:341`; `ArchiveStore` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:343-348`; `UnifiedArchiveResumeCheckpoint` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:351-356`; `LiveCompleteRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:359-373`; `CoverageRecordPayload` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:376-384`; `CoverageRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:387-390`; `LiveAttemptState` (enum, hors tests) `crates/whisper-adapters/src/archive.rs:394-398`; `LiveAttemptRecord` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:401-404`; `LiveAttemptPayload` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:407-414`; `LiveArchive` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:418-434`; `ActiveArchive` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:436-446`; `CapturedSourceManifest` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:449-456`; `CapturedPartReference` (struct, hors tests) `crates/whisper-adapters/src/archive.rs:459-466`.

Fonctions avec corps : `legacy_transcription_strategy` `crates/whisper-adapters/src/archive.rs:118-120`; `installed_export_ownership_path` `crates/whisper-adapters/src/archive.rs:208-210`; `cleanup_object_owned_default` `crates/whisper-adapters/src/archive.rs:282-284`; `default` `crates/whisper-adapters/src/archive.rs:321-334`; `browse_treatment_folder` `crates/whisper-adapters/src/archive.rs:470-481`; `browse_treatment_folder` `crates/whisper-adapters/src/archive.rs:484-486`; `new` `crates/whisper-adapters/src/archive.rs:488-495`; `take_unified_resume_checkpoint` `crates/whisper-adapters/src/archive.rs:497-499`; `root_path` `crates/whisper-adapters/src/archive.rs:500-502`; `next_segment_sequence` `crates/whisper-adapters/src/archive.rs:504-508`; `generation_path` `crates/whisper-adapters/src/archive.rs:511-515`; `load_or_migrate_publication_index` `crates/whisper-adapters/src/archive.rs:517-544`; `unix_time_ms` `crates/whisper-adapters/src/archive.rs:546-551`; `timestamp_checked` `crates/whisper-adapters/src/archive.rs:553-567`; `timestamp` `crates/whisper-adapters/src/archive.rs:569-578`.

Déclarations `pub` (visibilité sous conditions) : `pub struct PublicationReconfirmation<'a> {` `crates/whisper-adapters/src/archive.rs:18`; `pub use recovery::scan_live_recoveries;` `crates/whisper-adapters/src/archive.rs:79`; `pub use source::sha256_file;` `crates/whisper-adapters/src/archive.rs:84`; `pub struct SegmentRecord {` `crates/whisper-adapters/src/archive.rs:95`; `pub struct ArchiveQualificationKey {` `crates/whisper-adapters/src/archive.rs:152`; `pub struct ArchiveEntry {` `crates/whisper-adapters/src/archive.rs:213`; `pub struct PublicationLock {` `crates/whisper-adapters/src/archive.rs:337`; `pub struct ArchiveStore {` `crates/whisper-adapters/src/archive.rs:343`; `pub struct UnifiedArchiveResumeCheckpoint {` `crates/whisper-adapters/src/archive.rs:351`; `pub struct LiveArchive {` `crates/whisper-adapters/src/archive.rs:418`; `pub fn browse_treatment_folder(destination: &Path) -> Option<String> {` `crates/whisper-adapters/src/archive.rs:470`; `pub fn browse_treatment_folder(_destination: &Path) -> Option<String> {` `crates/whisper-adapters/src/archive.rs:484`; `pub fn new(root: impl Into<PathBuf>) -> Self {` `crates/whisper-adapters/src/archive.rs:488`; `pub fn take_unified_resume_checkpoint(&mut self) -> Option<UnifiedArchiveResumeCheckpoint> {` `crates/whisper-adapters/src/archive.rs:497`; `pub fn root_path(&self) -> PathBuf {` `crates/whisper-adapters/src/archive.rs:500`; `pub fn next_segment_sequence(&self) -> u64 {` `crates/whisper-adapters/src/archive.rs:504`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/archive.rs:1-581`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::capture — `crates/whisper-adapters/src/capture.rs`

Ouvre les entrées microphone et convertit les blocs avec compensation et gain. `crates/whisper-adapters/src/capture.rs:1-1186`

Mesure : 691 production / 495 tests / 1186 physiques. `crates/whisper-adapters/src/capture.rs:1-1186`

Types : `CaptureStatus` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:19-22`; `CapturedBlock` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:24-28`; `CaptureBuffer` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:45-50`; `CaptureStream` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:52-69`; `Pcm16Converter` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:71-82`; `ConvertedCaptureBlock` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:84-87`; `ProbeAudioBlock` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:89-98`; `CaptureCallbackState` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:263-270`; `WasapiMicrophonePort` (struct, hors tests) `crates/whisper-adapters/src/capture.rs:342`.

Fonctions avec corps : `samples` `crates/whisper-adapters/src/capture.rs:31-33`; `gain` `crates/whisper-adapters/src/capture.rs:34-36`; `first_frame` `crates/whisper-adapters/src/capture.rs:37-39`; `acquired_at` `crates/whisper-adapters/src/capture.rs:40-42`; `new` `crates/whisper-adapters/src/capture.rs:101-123`; `push` `crates/whisper-adapters/src/capture.rs:126-129`; `push_with_probe` `crates/whisper-adapters/src/capture.rs:131-136`; `push_inner` `crates/whisper-adapters/src/capture.rs:138-216`; `finish` `crates/whisper-adapters/src/capture.rs:221-247`; `collect_compensated` `crates/whisper-adapters/src/capture.rs:249-260`; `capture_into` `crates/whisper-adapters/src/capture.rs:272-326`; `start_default` `crates/whisper-adapters/src/capture.rs:328-330`; `start_default_with_gain` `crates/whisper-adapters/src/capture.rs:332-336`; `start_direct_default` `crates/whisper-adapters/src/capture.rs:338-340`; `default_input_identity` `crates/whisper-adapters/src/capture.rs:345-352`; `list_inputs` `crates/whisper-adapters/src/capture.rs:354-378`; `start_direct` `crates/whisper-adapters/src/capture.rs:381-383`; `start_direct_with_gain` `crates/whisper-adapters/src/capture.rs:385-409`; `capture_compatibility` `crates/whisper-adapters/src/capture.rs:411-428`; `wasapi_host` `crates/whisper-adapters/src/capture.rs:430-433`; `default_input_device` `crates/whisper-adapters/src/capture.rs:435-439`; `start_device` `crates/whisper-adapters/src/capture.rs:441-449`; `start_device_with_buffer` `crates/whisper-adapters/src/capture.rs:451-598`; `record_stream_error` `crates/whisper-adapters/src/capture.rs:600-615`; `from_test_blocks` [tests] `crates/whisper-adapters/src/capture.rs:619-655`; `empty_test_capture` [tests] `crates/whisper-adapters/src/capture.rs:658-686`; `empty_test_capture_with_gain` [tests] `crates/whisper-adapters/src/capture.rs:689-701`; `enqueue_test_block` [tests] `crates/whisper-adapters/src/capture.rs:704-726`; `first_sample_at_unix_ms` `crates/whisper-adapters/src/capture.rs:728-731`; `sample_rate_hz` `crates/whisper-adapters/src/capture.rs:732-734`; `channels` `crates/whisper-adapters/src/capture.rs:735-737`; `source_identity` `crates/whisper-adapters/src/capture.rs:738-740`; `current_gain` `crates/whisper-adapters/src/capture.rs:741-743`; `set_gain` `crates/whisper-adapters/src/capture.rs:744-761`; `try_next_block` `crates/whisper-adapters/src/capture.rs:762-768`; `recycle_block` `crates/whisper-adapters/src/capture.rs:769-774`; `is_saturated` `crates/whisper-adapters/src/capture.rs:775-777`; `failure` `crates/whisper-adapters/src/capture.rs:778-780`; `take_xruns` `crates/whisper-adapters/src/capture.rs:781-783`; `failure_detail` `crates/whisper-adapters/src/capture.rs:784-789`; `stop` `crates/whisper-adapters/src/capture.rs:790-792`; `pack_gain` `crates/whisper-adapters/src/capture.rs:795-800`; `unpack_gain` `crates/whisper-adapters/src/capture.rs:802-808`; `empty_and_exact_chunk_capture_finish_without_an_empty_input_buffer` [tests] `crates/whisper-adapters/src/capture.rs:815-837`; `callback_reuses_recycled_slots_without_blocking_or_saturating` [tests] `crates/whisper-adapters/src/capture.rs:840-881`; `callback_marks_partial_channel_frames_as_failed` [tests] `crates/whisper-adapters/src/capture.rs:884-914`; `callback_stamps_each_block_with_the_gain_active_at_its_acquisition_boundary` [tests] `crates/whisper-adapters/src/capture.rs:917-975`; `gain_published_during_sample_copy_belongs_to_the_next_callback_block` [tests] `crates/whisper-adapters/src/capture.rs:978-1034`; `gain_is_applied_after_mono_measurement_and_before_limiter` [tests] `crates/whisper-adapters/src/capture.rs:1037-1059`; `neutral_gain_keeps_pre_gain_values_and_only_counts_the_existing_limiter` [tests] `crates/whisper-adapters/src/capture.rs:1062-1081`; `collecting_probe_data_does_not_change_converted_or_drained_audio` [tests] `crates/whisper-adapters/src/capture.rs:1084-1103`; `stale_gain_revision_is_rejected_without_replacing_the_current_value` [tests] `crates/whisper-adapters/src/capture.rs:1106-1112`; `wasapi_xrun_is_counted_without_terminating_the_stream` [tests] `crates/whisper-adapters/src/capture.rs:1115-1142`; `native_wasapi_input_catalog_has_stable_parseable_ids` [tests] `crates/whisper-adapters/src/capture.rs:1147-1162`; `native_selected_microphone_survives_initial_discontinuity` [tests] `crates/whisper-adapters/src/capture.rs:1167-1185`.

Déclarations `pub` (visibilité sous conditions) : `pub const CAPTURE_SLOTS: usize = 100;` `crates/whisper-adapters/src/capture.rs:14`; `pub struct CapturedBlock {` `crates/whisper-adapters/src/capture.rs:24`; `pub fn samples(&self) -> &[f32] {` `crates/whisper-adapters/src/capture.rs:31`; `pub const fn gain(&self) -> GainRevision {` `crates/whisper-adapters/src/capture.rs:34`; `pub const fn first_frame(&self) -> u64 {` `crates/whisper-adapters/src/capture.rs:37`; `pub const fn acquired_at(&self) -> std::time::Instant {` `crates/whisper-adapters/src/capture.rs:40`; `pub struct CaptureStream {` `crates/whisper-adapters/src/capture.rs:52`; `pub struct Pcm16Converter {` `crates/whisper-adapters/src/capture.rs:71`; `pub struct ConvertedCaptureBlock {` `crates/whisper-adapters/src/capture.rs:84`; `pub struct ProbeAudioBlock {` `crates/whisper-adapters/src/capture.rs:89`; `pub fn new(rate: u32, channels: u16) -> Result<Self, String> {` `crates/whisper-adapters/src/capture.rs:101`; `pub fn push(&mut self, block: &CapturedBlock) -> Result<Vec<i16>, String> {` `crates/whisper-adapters/src/capture.rs:126`; `pub fn push_with_probe(` `crates/whisper-adapters/src/capture.rs:131`; `pub fn finish(&mut self) -> Result<Vec<i16>, String> {` `crates/whisper-adapters/src/capture.rs:221`; `pub fn start_default() -> Result<CaptureStream, String> {` `crates/whisper-adapters/src/capture.rs:328`; `pub fn start_default_with_gain(` `crates/whisper-adapters/src/capture.rs:332`; `pub fn start_direct_default() -> Result<CaptureStream, String> {` `crates/whisper-adapters/src/capture.rs:338`; `pub struct WasapiMicrophonePort;` `crates/whisper-adapters/src/capture.rs:342`; `pub fn start_direct(choice: &MicrophoneChoice) -> Result<CaptureStream, String> {` `crates/whisper-adapters/src/capture.rs:381`; `pub fn start_direct_with_gain(` `crates/whisper-adapters/src/capture.rs:385`; `pub fn start_device(device: Device) -> Result<CaptureStream, String> {` `crates/whisper-adapters/src/capture.rs:441`; `pub fn first_sample_at_unix_ms(&self) -> Option<u64> {` `crates/whisper-adapters/src/capture.rs:728`; `pub fn sample_rate_hz(&self) -> u32 {` `crates/whisper-adapters/src/capture.rs:732`; `pub fn channels(&self) -> u16 {` `crates/whisper-adapters/src/capture.rs:735`; `pub fn source_identity(&self) -> Option<&str> {` `crates/whisper-adapters/src/capture.rs:738`; `pub fn current_gain(&self) -> GainRevision {` `crates/whisper-adapters/src/capture.rs:741`; `pub fn set_gain(&self, expected_revision: u64, gain: GainDb) -> Result<GainRevision, String> {` `crates/whisper-adapters/src/capture.rs:744`; `pub fn try_next_block(&mut self) -> Option<CapturedBlock> {` `crates/whisper-adapters/src/capture.rs:762`; `pub fn recycle_block(&mut self, mut block: CapturedBlock) -> Result<(), String> {` `crates/whisper-adapters/src/capture.rs:769`; `pub fn is_saturated(&self) -> bool {` `crates/whisper-adapters/src/capture.rs:775`; `pub fn failure(&self) -> bool {` `crates/whisper-adapters/src/capture.rs:778`; `pub fn take_xruns(&self) -> u64 {` `crates/whisper-adapters/src/capture.rs:781`; `pub fn failure_detail(&self) -> Option<String> {` `crates/whisper-adapters/src/capture.rs:784`; `pub fn stop(&mut self) {` `crates/whisper-adapters/src/capture.rs:790`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/capture.rs:1-1186`.

Utilisé par, via ces imports : tests : `crate::direct_live::d2_corpus` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::microphone_probe` `crates/whisper-adapters/src/microphone_probe.rs:21`<br>tests : `crate::microphone_probe` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>tests : `crate::microphone_probe` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>tests : `crate::microphone_probe` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`.

#### Inline `tests`

Tests/auxiliaires : `empty_and_exact_chunk_capture_finish_without_an_empty_input_buffer` `crates/whisper-adapters/src/capture.rs:815-837`, `callback_reuses_recycled_slots_without_blocking_or_saturating` `crates/whisper-adapters/src/capture.rs:840-881`, `callback_marks_partial_channel_frames_as_failed` `crates/whisper-adapters/src/capture.rs:884-914`, `callback_stamps_each_block_with_the_gain_active_at_its_acquisition_boundary` `crates/whisper-adapters/src/capture.rs:917-975`. `crates/whisper-adapters/src/capture.rs:810-1186`

Mesure locale : 0 production / 377 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/capture.rs:810-1186`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/capture.rs:810-1186`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/capture.rs:810-1186`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::decoder — `crates/whisper-adapters/src/decoder.rs`

Valide une demande de décodage et la transmet par WorkerTransport. `crates/whisper-adapters/src/decoder.rs:1-251`

Mesure : 103 production / 148 tests / 251 physiques. `crates/whisper-adapters/src/decoder.rs:1-251`

Types : `DecoderProxy` (struct, hors tests) `crates/whisper-adapters/src/decoder.rs:9-12`; `Stub` (struct, tests) `crates/whisper-adapters/src/decoder.rs:113`.

Fonctions avec corps : `new` `crates/whisper-adapters/src/decoder.rs:15-20`; `into_inner` `crates/whisper-adapters/src/decoder.rs:22-24`; `decode` `crates/whisper-adapters/src/decoder.rs:28-87`; `range_is_within` `crates/whisper-adapters/src/decoder.rs:90-102`; `request` [tests] `crates/whisper-adapters/src/decoder.rs:115-117`; `receive` [tests] `crates/whisper-adapters/src/decoder.rs:118-120`; `valid_response` [tests] `crates/whisper-adapters/src/decoder.rs:123-142`; `decoder_bounds_accept_one_and_eighty_thousand_and_reject_zero_or_oversized` [tests] `crates/whisper-adapters/src/decoder.rs:145-183`; `decoder_rejects_a_block_outside_the_requested_window` [tests] `crates/whisper-adapters/src/decoder.rs:186-216`; `decoder_rejects_a_partial_block_that_skips_the_requested_prefix` [tests] `crates/whisper-adapters/src/decoder.rs:219-250`.

Déclarations `pub` (visibilité sous conditions) : `pub struct DecoderProxy<T> {` `crates/whisper-adapters/src/decoder.rs:9`; `pub fn new(transport: T) -> Self {` `crates/whisper-adapters/src/decoder.rs:15`; `pub fn into_inner(self) -> T {` `crates/whisper-adapters/src/decoder.rs:22`.

Dépend de, via `use crate::` : hors tests : `crate::WorkerTransport` → [Non vérifié] symbole/réexport non résolu `crates/whisper-adapters/src/decoder.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `request` `crates/whisper-adapters/src/decoder.rs:115-117`, `receive` `crates/whisper-adapters/src/decoder.rs:118-120`, `valid_response` `crates/whisper-adapters/src/decoder.rs:123-142`, `decoder_bounds_accept_one_and_eighty_thousand_and_reject_zero_or_oversized` `crates/whisper-adapters/src/decoder.rs:145-183`. `crates/whisper-adapters/src/decoder.rs:104-251`

Mesure locale : 0 production / 148 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/decoder.rs:104-251`

Types locaux : `Stub` `crates/whisper-adapters/src/decoder.rs:113`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/decoder.rs:104-251`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::direct_live::capture — `crates/whisper-adapters/src/direct_live/capture.rs`

Draine les blocs capturés et publie les mesures niveau/spectre. `crates/whisper-adapters/src/direct_live/capture.rs:1-148`

Mesure : 148 production / 0 tests / 148 physiques. `crates/whisper-adapters/src/direct_live/capture.rs:1-148`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/direct_live/capture.rs:1-148`.

Fonctions avec corps : `take_next_probe_telemetry` `crates/whisper-adapters/src/direct_live/capture.rs:4-14`; `pump_capture` `crates/whisper-adapters/src/direct_live/capture.rs:16-21`; `take_probe_meter` `crates/whisper-adapters/src/direct_live/capture.rs:23-31`; `take_probe_spectrum` `crates/whisper-adapters/src/direct_live/capture.rs:33-37`; `drain_capture` `crates/whisper-adapters/src/direct_live/capture.rs:39-105`; `record_capture_issue` `crates/whisper-adapters/src/direct_live/capture.rs:107-133`; `microphone_error` `crates/whisper-adapters/src/direct_live/capture.rs:135-147`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/capture.rs:1-148`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/direct_live/capture.rs:1-148`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::direct_live::d2_corpus — `crates/whisper-adapters/src/direct_live/d2_corpus.rs`

Tests ou supports : définit `d2_corpus_real_cpu_turbo_and_parakeet` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:143-282`, `d2_frame_deadlines_follow_the_monotone_one_x_sample_clock` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:455-468`.

Mesure : 0 production / 622 tests / 622 physiques. `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1-622`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1-622`.

Fonctions avec corps : `d2_frame_deadline` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:59-66`; `d2_rebase_if_late` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:68-76`; `drain_d2_events` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:78-89`; `pace_d2_frame_end` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:91-111`; `record_parakeet_poll_event` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:113-128`; `drain_d2_events_after_playback` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:130-140`; `d2_corpus_real_cpu_turbo_and_parakeet` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:143-282`; `turbo_case` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:284-348`; `parakeet_case` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:350-444`; `record_engine_status` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:446-448`; `d2_frame_deadlines_follow_the_monotone_one_x_sample_clock` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:455-468`; `d2_event_drain_stops_after_its_bounded_batch` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:471-488`; `d2_driver_paces_frames_and_drains_events_before_deadline` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:491-507`; `d2_expired_frame_deadline_rebases_without_catch_up_burst` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:510-521`; `d2_post_playback_drain_has_a_bounded_batch_count` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:524-537`; `test_status_writer_preserves_diagnostics` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:540-558`; `read_wav16` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:561-600`; `pcm_frames` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:601-605`; `hash` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:606-608`; `decode_sha256` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:609-612`; `record` [tests] `crates/whisper-adapters/src/direct_live/d2_corpus.rs:613-622`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1-622`.

Dépend de, via `use crate::` : tests : `crate::capture::CaptureStream` → `crate::capture` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1`<br>tests : `crate::direct_live::DirectLive` → `crate::direct_live` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1`<br>tests : `crate::parakeet_import::ParakeetWorkerPaths` → `crate::parakeet_import` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `d2_frame_deadlines_follow_the_monotone_one_x_sample_clock` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:455-468`, `d2_event_drain_stops_after_its_bounded_batch` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:471-488`, `d2_driver_paces_frames_and_drains_events_before_deadline` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:491-507`, `d2_expired_frame_deadline_rebases_without_catch_up_burst` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:510-521`. `crates/whisper-adapters/src/direct_live/d2_corpus.rs:450-559`

Mesure locale : 0 production / 110 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/direct_live/d2_corpus.rs:450-559`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/direct_live/d2_corpus.rs:450-559`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/direct_live/d2_corpus.rs:450-559`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::direct_live::outputs — `crates/whisper-adapters/src/direct_live/outputs.rs`

Écrit les exports texte/SRT et les manifestes capture/finalisation Parakeet. `crates/whisper-adapters/src/direct_live/outputs.rs:1-267`

Mesure : 267 production / 0 tests / 267 physiques. `crates/whisper-adapters/src/direct_live/outputs.rs:1-267`

Types : `ParakeetExportOutcome` (struct, hors tests) `crates/whisper-adapters/src/direct_live/outputs.rs:204-209`.

Fonctions avec corps : `finalize_incomplete_outputs` `crates/whisper-adapters/src/direct_live/outputs.rs:3-30`; `export_segments` `crates/whisper-adapters/src/direct_live/outputs.rs:32-118`; `automatic_final_output_paths` `crates/whisper-adapters/src/direct_live/outputs.rs:120-143`; `export_parakeet_manifest` `crates/whisper-adapters/src/direct_live/outputs.rs:145-202`; `export_parakeet_outputs` `crates/whisper-adapters/src/direct_live/outputs.rs:211-256`; `timestamp` `crates/whisper-adapters/src/direct_live/outputs.rs:258-267`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/outputs.rs:1-267`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/direct_live/outputs.rs:1-267`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::direct_live::test_support — `crates/whisper-adapters/src/direct_live/test_support.rs`

Tests ou supports : définit `from_test_capture` `crates/whisper-adapters/src/direct_live/test_support.rs:5-53`, `from_test_capture_with_cpu_worker` `crates/whisper-adapters/src/direct_live/test_support.rs:56-110`.

Mesure : 0 production / 129 tests / 129 physiques. `crates/whisper-adapters/src/direct_live/test_support.rs:1-129`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/direct_live/test_support.rs:1-129`.

Fonctions avec corps : `from_test_capture` [tests] `crates/whisper-adapters/src/direct_live/test_support.rs:5-53`; `from_test_capture_with_cpu_worker` [tests] `crates/whisper-adapters/src/direct_live/test_support.rs:56-110`; `finish_test_wav` [tests] `crates/whisper-adapters/src/direct_live/test_support.rs:113-115`; `test_worker_ready` [tests] `crates/whisper-adapters/src/direct_live/test_support.rs:118-120`; `enqueue_test_capture_block` [tests] `crates/whisper-adapters/src/direct_live/test_support.rs:123-128`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/test_support.rs:1-129`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/direct_live/test_support.rs:1-129`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::direct_live::tests — `crates/whisper-adapters/src/direct_live/tests.rs`

Tests ou supports : définit `direct_capture_publishes_frequency_spectrum_without_changing_wav_audio` `crates/whisper-adapters/src/direct_live/tests.rs:26-112`, `direct_capture_poll_keeps_spectrum_text_and_terminal_progress_within_four_events` `crates/whisper-adapters/src/direct_live/tests.rs:115-209`.

Mesure : 0 production / 1262 tests / 1262 physiques. `crates/whisper-adapters/src/direct_live/tests.rs:1-1262`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/direct_live/tests.rs:1-1262`.

Fonctions avec corps : `test_directory` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:14-23`; `direct_capture_publishes_frequency_spectrum_without_changing_wav_audio` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:26-112`; `direct_capture_poll_keeps_spectrum_text_and_terminal_progress_within_four_events` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:115-209`; `parakeet_live_automatic_final_paths_are_distinct_and_explicitly_unverified` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:212-224`; `cpu_ready_after_auto_fallback_needs_no_gpu_candidate_attestation` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:227-267`; `startup_request` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:269-288`; `startup_gpu_candidate` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:290-300`; `startup_worker` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:302-392`; `auto_microphone_falls_back_while_capture_is_open_and_keeps_auto_preference` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:395-490`; `auto_without_gpu_worker_clears_candidate_and_accepts_cpu_ready` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:493-527`; `auto_microphone_stop_before_gpu_ready_keeps_cpu_retry_for_recorded_audio` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:530-568`; `wav_header_describes_mono_16khz_pcm_and_exact_audio_length` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:571-590`; `wav_output_keeps_the_exact_converted_samples` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:593-628`; `stop_before_engine_ready_flushes_captured_audio_to_the_wav` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:631-701`; `gain_command_changes_only_subsequently_acquired_audio_and_stop_finalizes_wav` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:704-780`; `test_capture_gain_preserves_probe_a_and_applies_once_to_wav_pcm` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:783-840`; `wav_backlog_fills_bounded_windows_without_drops_or_reordering` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:843-874`; `simulated_gpu_failure_keeps_wav_and_confirmed_text_with_incomplete_suffix` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:877-938`; `microphone_interruption_finalizes_cpu_outputs_without_gpu_message` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:941-970`; `live_window_waits_for_five_seconds_then_prefers_a_pause_before_eight` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:973-989`; `parakeet_exports_sort_time_and_write_versioned_companion_intervals` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:992-1045`; `parakeet_srt_failure_removes_created_txt_and_retries_coherent_incomplete_outputs` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:1048-1100`; `worker_rejects_stale_ready_and_confirms_segments_only_after_matching_window_end` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:1103-1170`; `gpu_ready_requires_matching_attestation_and_stale_failed_keeps_existing_error` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:1173-1233`; `parakeet_json_write_failure_preserves_text_outputs_and_reports_incomplete` [tests] `crates/whisper-adapters/src/direct_live/tests.rs:1236-1262`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/tests.rs:1-1262`.

Dépend de, via `use crate::` : tests : `crate::microphone_probe::MicrophoneProbe` → `crate::microphone_probe` `crates/whisper-adapters/src/direct_live/tests.rs:8`<br>tests : `crate::microphone_probe::MicrophoneSpectrumReader` → `crate::microphone_probe` `crates/whisper-adapters/src/direct_live/tests.rs:8`<br>tests : `crate::microphone_probe` → `crate::microphone_probe` `crates/whisper-adapters/src/direct_live/tests.rs:8`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::direct_live::wav — `crates/whisper-adapters/src/direct_live/wav.rs`

Écrit le WAV de capture et relit les plages de fenêtres en attente. `crates/whisper-adapters/src/direct_live/wav.rs:1-134`

Mesure : 134 production / 0 tests / 134 physiques. `crates/whisper-adapters/src/direct_live/wav.rs:1-134`

Types : `WavOutput` (struct, hors tests) `crates/whisper-adapters/src/direct_live/wav.rs:20-24`.

Fonctions avec corps : `fill_pending_from_wav` `crates/whisper-adapters/src/direct_live/wav.rs:3-18`; `create` `crates/whisper-adapters/src/direct_live/wav.rs:27-44`; `append` `crates/whisper-adapters/src/direct_live/wav.rs:46-63`; `read_samples` `crates/whisper-adapters/src/direct_live/wav.rs:65-91`; `sample_count` `crates/whisper-adapters/src/direct_live/wav.rs:93-95`; `finish` `crates/whisper-adapters/src/direct_live/wav.rs:97-106`; `drop` `crates/whisper-adapters/src/direct_live/wav.rs:110-115`; `write_wav_header` `crates/whisper-adapters/src/direct_live/wav.rs:118-134`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/wav.rs:1-134`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/direct_live/wav.rs:1-134`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::direct_live::worker — `crates/whisper-adapters/src/direct_live/worker.rs`

Traite les événements du worker direct et le repli CPU avant disponibilité. `crates/whisper-adapters/src/direct_live/worker.rs:1-336`

Mesure : 336 production / 0 tests / 336 physiques. `crates/whisper-adapters/src/direct_live/worker.rs:1-336`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/direct_live/worker.rs:1-336`.

Fonctions avec corps : `handle_worker` `crates/whisper-adapters/src/direct_live/worker.rs:4-213`; `engine_name` `crates/whisper-adapters/src/direct_live/worker.rs:215-217`; `retry_cpu` `crates/whisper-adapters/src/direct_live/worker.rs:219-245`; `backend_kind` `crates/whisper-adapters/src/direct_live/worker.rs:247-253`; `matches` `crates/whisper-adapters/src/direct_live/worker.rs:255-257`; `gpu_ready_requires_attestation` `crates/whisper-adapters/src/direct_live/worker.rs:260-262`; `should_retry_auto_gpu_before_ready` `crates/whisper-adapters/src/direct_live/worker.rs:264-280`; `clear_gpu_proof_for_cpu` `crates/whisper-adapters/src/direct_live/worker.rs:282-288`; `spawn_direct_worker` `crates/whisper-adapters/src/direct_live/worker.rs:290-328`; `compute_name` `crates/whisper-adapters/src/direct_live/worker.rs:330-336`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live/worker.rs:1-336`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/direct_live/worker.rs:1-336`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::direct_live — `crates/whisper-adapters/src/direct_live.rs`

Porte la session micro Turbo et pilote capture, worker, fenêtres et export. `crates/whisper-adapters/src/direct_live.rs:1-470`

Mesure : 458 production / 12 tests / 470 physiques. `crates/whisper-adapters/src/direct_live.rs:1-470`

Types : `DirectLive` (struct, hors tests) `crates/whisper-adapters/src/direct_live.rs:59-94`.

Fonctions avec corps : `start` `crates/whisper-adapters/src/direct_live.rs:97-250`; `identity` `crates/whisper-adapters/src/direct_live.rs:252-254`; `set_gain` `crates/whisper-adapters/src/direct_live.rs:256-267`; `done` `crates/whisper-adapters/src/direct_live.rs:269-271`; `take_idle_worker` `crates/whisper-adapters/src/direct_live.rs:273-275`; `cache_identity` `crates/whisper-adapters/src/direct_live.rs:277-283`; `stop` `crates/whisper-adapters/src/direct_live.rs:285-302`; `poll` `crates/whisper-adapters/src/direct_live.rs:304-357`; `send_next` `crates/whisper-adapters/src/direct_live.rs:359-402`; `fill_pending_from_wav` `crates/whisper-adapters/src/direct_live.rs:404-409`; `fail` `crates/whisper-adapters/src/direct_live.rs:411-438`; `export` `crates/whisper-adapters/src/direct_live.rs:440-442`; `window_size` `crates/whisper-adapters/src/direct_live.rs:445-462`; `drop` `crates/whisper-adapters/src/direct_live.rs:465-469`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/direct_live.rs:1-470`.

Dépend de, via `use crate::` : hors tests : `crate::WorkerTransport` → [Non vérifié] symbole/réexport non résolu `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::capture::CaptureStream` → `crate::capture` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::capture::Pcm16Converter` → `crate::capture` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::capture` → `crate::capture` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::microphone_probe::MicrophoneProbe` → `crate::microphone_probe` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::microphone_probe::MicrophoneSpectrumReader` → `crate::microphone_probe` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::microphone_probe` → `crate::microphone_probe` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::supervisor::DiagnosticCode` → `crate::supervisor` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::supervisor::DiagnosticPhase` → `crate::supervisor` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::supervisor::DiagnosticRecord` → `crate::supervisor` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::supervisor::DiagnosticSink` → `crate::supervisor` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::supervisor::WorkerPaths` → `crate::supervisor` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::supervisor` → `crate::supervisor` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::worker_ipc::ChildWorkerTransport` → `crate::worker_ipc` `crates/whisper-adapters/src/direct_live.rs:42`.

Utilisé par, via ces imports : tests : `crate::direct_live::d2_corpus` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1`<br>tests : `crate::microphone_probe` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`.

### crate::gpu_inventory::windows_native — `crates/whisper-adapters/src/gpu_inventory/windows_native.rs`

Énumère les adaptateurs DXGI et leurs identifiants Windows. `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:1-161`

Mesure : 139 production / 22 tests / 161 physiques. `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:1-161`

Types : `DxgiAdapter` (struct, hors tests) `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:22-25`.

Fonctions avec corps : `luid_u64` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:27-29`; `adapter_instance_id` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:31-111`; `enumerate_dxgi` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:115-138`; `live_dxgi_enumeration_has_ordered_ranks_and_setupapi_identity_join` [tests] `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:145-160`.

Déclarations `pub` (visibilité sous conditions) : `pub struct DxgiAdapter {` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:22`; `pub fn luid_u64(luid: LUID) -> u64 {` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:27`; `pub fn adapter_instance_id(luid: LUID) -> Result<String, String> {` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:31`; `pub fn enumerate_dxgi() -> Result<Vec<DxgiAdapter>, String> {` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:115`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:1-161`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `live_dxgi_enumeration_has_ordered_ranks_and_setupapi_identity_join` `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:145-160`. `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:140-161`

Mesure locale : 0 production / 22 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:140-161`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:140-161`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:140-161`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::gpu_inventory — `crates/whisper-adapters/src/gpu_inventory.rs`

Collecte les observations Windows/GPU et les preuves du worker. `crates/whisper-adapters/src/gpu_inventory.rs:1-855`

Mesure : 613 production / 242 tests / 855 physiques. `crates/whisper-adapters/src/gpu_inventory.rs:1-855`

Types : `RuntimeInventoryResult` (type, hors tests) `crates/whisper-adapters/src/gpu_inventory.rs:18-21`; `RuntimeInventoryReceiver` (type, hors tests) `crates/whisper-adapters/src/gpu_inventory.rs:22`; `DxgiLogEntry` (struct, hors tests) `crates/whisper-adapters/src/gpu_inventory.rs:25-30`; `WindowsGpuInventoryPort` (struct, hors tests) `crates/whisper-adapters/src/gpu_inventory.rs:38-45`; `InventoryLogSnapshot` (struct, hors tests) `crates/whisper-adapters/src/gpu_inventory.rs:330-338`.

Fonctions avec corps : `gpu_query` `crates/whisper-adapters/src/gpu_inventory.rs:32-36`; `new` `crates/whisper-adapters/src/gpu_inventory.rs:48-53`; `map_device_fields` `crates/whisper-adapters/src/gpu_inventory.rs:56-83`; `default` `crates/whisper-adapters/src/gpu_inventory.rs:86-95`; `start` `crates/whisper-adapters/src/gpu_inventory.rs:99-229`; `uses_qualified_catalogue` `crates/whisper-adapters/src/gpu_inventory.rs:231-233`; `poll_qualified` `crates/whisper-adapters/src/gpu_inventory.rs:235-250`; `poll_runtime_identities` `crates/whisper-adapters/src/gpu_inventory.rs:252-272`; `poll` `crates/whisper-adapters/src/gpu_inventory.rs:274-290`; `log_inventory` `crates/whisper-adapters/src/gpu_inventory.rs:293-328`; `append_inventory_log` `crates/whisper-adapters/src/gpu_inventory.rs:340-405`; `qualified_collection` `crates/whisper-adapters/src/gpu_inventory.rs:407-494`; `runtime_identities` `crates/whisper-adapters/src/gpu_inventory.rs:496-517`; `unqualified_collection` `crates/whisper-adapters/src/gpu_inventory.rs:519-536`; `unique_adapter` `crates/whisper-adapters/src/gpu_inventory.rs:538-544`; `collect_windows` `crates/whisper-adapters/src/gpu_inventory.rs:547-586`; `collect_windows` `crates/whisper-adapters/src/gpu_inventory.rs:589-591`; `variant_string` `crates/whisper-adapters/src/gpu_inventory.rs:594-599`; `variant_u32` `crates/whisper-adapters/src/gpu_inventory.rs:602-612`; `query_uses_the_display_class_guid_without_filtering_current_presence` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:627-633`; `missing_dxgi_identity_match_is_safe_and_returns_no_adapter` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:636-682`; `dxgi_failure_keeps_wmi_observations_as_partial_catalogue_with_cause` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:685-710`; `runtime_proof_keeps_windows_rank_distinct_from_cuda_and_ggml_ordinals` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:713-742`; `inventory_log_appends_timestamped_partial_snapshots_and_escapes_names` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:745-790`; `inventory_log_write_failure_is_reportable_without_panicking` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:793-811`; `inventory_log_keeps_dxgi_adapters_that_failed_identity_join` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:814-841`; `inventory_test_path` [tests] `crates/whisper-adapters/src/gpu_inventory.rs:843-854`.

Déclarations `pub` (visibilité sous conditions) : `pub struct WindowsGpuInventoryPort {` `crates/whisper-adapters/src/gpu_inventory.rs:38`; `pub fn new(worker: PathBuf) -> Self {` `crates/whisper-adapters/src/gpu_inventory.rs:48`; `pub fn map_device_fields(` `crates/whisper-adapters/src/gpu_inventory.rs:56`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/gpu_inventory.rs:1-855`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `query_uses_the_display_class_guid_without_filtering_current_presence` `crates/whisper-adapters/src/gpu_inventory.rs:627-633`, `missing_dxgi_identity_match_is_safe_and_returns_no_adapter` `crates/whisper-adapters/src/gpu_inventory.rs:636-682`, `dxgi_failure_keeps_wmi_observations_as_partial_catalogue_with_cause` `crates/whisper-adapters/src/gpu_inventory.rs:685-710`, `runtime_proof_keeps_windows_rank_distinct_from_cuda_and_ggml_ordinals` `crates/whisper-adapters/src/gpu_inventory.rs:713-742`. `crates/whisper-adapters/src/gpu_inventory.rs:614-855`

Mesure locale : 0 production / 242 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/gpu_inventory.rs:614-855`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/gpu_inventory.rs:614-855`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/gpu_inventory.rs:614-855`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::journal — `crates/whisper-adapters/src/journal.rs`

Réexporte SegmentRecord depuis archive. `crates/whisper-adapters/src/journal.rs:1-3`

Mesure : 3 production / 0 tests / 3 physiques. `crates/whisper-adapters/src/journal.rs:1-3`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/journal.rs:1-3`.

Fonctions avec corps : aucune relevée dans `crates/whisper-adapters/src/journal.rs:1-3`.

Déclarations `pub` (visibilité sous conditions) : `pub use crate::archive::SegmentRecord;` `crates/whisper-adapters/src/journal.rs:3`.

Dépend de, via `use crate::` : hors tests : `crate::archive::SegmentRecord` → `crate::archive` `crates/whisper-adapters/src/journal.rs:3`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate — `crates/whisper-adapters/src/lib.rs`

Déclare les modules et interfaces de bibliothèque. `crates/whisper-adapters/src/lib.rs:1-34`

Mesure : 34 production / 0 tests / 34 physiques. `crates/whisper-adapters/src/lib.rs:1-34`

Types : `WorkerTransport` (trait, hors tests) `crates/whisper-adapters/src/lib.rs:26-34`.

Fonctions avec corps : aucune relevée dans `crates/whisper-adapters/src/lib.rs:1-34`.

Déclarations `pub` (visibilité sous conditions) : `pub mod archive;` `crates/whisper-adapters/src/lib.rs:1`; `pub mod capture;` `crates/whisper-adapters/src/lib.rs:2`; `pub mod decoder;` `crates/whisper-adapters/src/lib.rs:3`; `pub mod gpu_inventory;` `crates/whisper-adapters/src/lib.rs:5`; `pub mod journal;` `crates/whisper-adapters/src/lib.rs:6`; `pub mod microphone_probe;` `crates/whisper-adapters/src/lib.rs:7`; `pub mod model_catalog;` `crates/whisper-adapters/src/lib.rs:8`; `pub mod parakeet_audio;` `crates/whisper-adapters/src/lib.rs:9`; `pub mod parakeet_document;` `crates/whisper-adapters/src/lib.rs:10`; `pub mod parakeet_import;` `crates/whisper-adapters/src/lib.rs:11`; `pub mod parakeet_pipeline;` `crates/whisper-adapters/src/lib.rs:13`; `pub mod path_settings;` `crates/whisper-adapters/src/lib.rs:14`; `pub mod queue_store;` `crates/whisper-adapters/src/lib.rs:15`; `pub mod recovery;` `crates/whisper-adapters/src/lib.rs:16`; `pub mod source_binding;` `crates/whisper-adapters/src/lib.rs:17`; `pub mod staging;` `crates/whisper-adapters/src/lib.rs:18`; `pub mod supervisor;` `crates/whisper-adapters/src/lib.rs:19`; `pub mod vad;` `crates/whisper-adapters/src/lib.rs:20`; `pub mod worker_ipc;` `crates/whisper-adapters/src/lib.rs:23`; `pub trait WorkerTransport {` `crates/whisper-adapters/src/lib.rs:26`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/lib.rs:1-34`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::microphone_probe::retry_epoch_regression — `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs`

Tests ou supports : définit `full_queue_invalidates_old_epoch_before_fresh_contiguous_audio` `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:17-69`.

Mesure : 0 production / 69 tests / 69 physiques. `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:1-69`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:1-69`.

Fonctions avec corps : `block` [tests] `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:3-14`; `full_queue_invalidates_old_epoch_before_fresh_contiguous_audio` [tests] `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:17-69`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:1-69`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/microphone_probe/retry_epoch_regression.rs:1-69`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::microphone_probe — `crates/whisper-adapters/src/microphone_probe.rs`

Calcule niveaux/spectres dans un thread avec contrôle d’époque. `crates/whisper-adapters/src/microphone_probe.rs:1-1252`

Mesure : 690 production / 562 tests / 1252 physiques. `crates/whisper-adapters/src/microphone_probe.rs:1-1252`

Types : `SpectralDensity` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:35-41`; `ProbeSnapshot` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:44-70`; `CachedSpectrum` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:73-83`; `MicrophoneSpectrumReader` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:88-92`; `ProbeChunk` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:191-199`; `MicrophoneProbe` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:201-216`; `ProbeSample` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:452-457`; `ProbeAnalyzer` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:459-480`; `CachedRms` (struct, hors tests) `crates/whisper-adapters/src/microphone_probe.rs:483-492`.

Fonctions avec corps : `poll` `crates/whisper-adapters/src/microphone_probe.rs:95-187`; `start` `crates/whisper-adapters/src/microphone_probe.rs:219-293`; `submit` `crates/whisper-adapters/src/microphone_probe.rs:296-340`; `latest_at` `crates/whisper-adapters/src/microphone_probe.rs:342-356`; `latest_spectrum_at` `crates/whisper-adapters/src/microphone_probe.rs:358-369`; `epoch` `crates/whisper-adapters/src/microphone_probe.rs:371-373`; `advance_epoch` `crates/whisper-adapters/src/microphone_probe.rs:376-389`; `dropped_updates` `crates/whisper-adapters/src/microphone_probe.rs:391-393`; `processed_chunks` [tests] `crates/whisper-adapters/src/microphone_probe.rs:396-398`; `invalidate` `crates/whisper-adapters/src/microphone_probe.rs:401-404`; `mark_dropped` `crates/whisper-adapters/src/microphone_probe.rs:406-410`; `drop` `crates/whisper-adapters/src/microphone_probe.rs:414-420`; `epoch_is_current` `crates/whisper-adapters/src/microphone_probe.rs:424-426`; `meter` `crates/whisper-adapters/src/microphone_probe.rs:428-449`; `new` `crates/whisper-adapters/src/microphone_probe.rs:495-530`; `reset` `crates/whisper-adapters/src/microphone_probe.rs:532-540`; `push` `crates/whisper-adapters/src/microphone_probe.rs:542-673`; `psd` `crates/whisper-adapters/src/microphone_probe.rs:675-708`; `gain` [tests] `crates/whisper-adapters/src/microphone_probe.rs:719-724`; `probe_and_spectrum_reader_leave_stt_pcm_and_final_wav_byte_identical` [tests] `crates/whisper-adapters/src/microphone_probe.rs:727-819`; `repeated_fresh_spectrum_snapshot_does_not_emit_unavailable` [tests] `crates/whisper-adapters/src/microphone_probe.rs:822-876`; `retry_epoch_discards_audio_acquired_before_the_retry_barrier` [tests] `crates/whisper-adapters/src/microphone_probe.rs:879-959`; `rms_peak_and_periodic_hann_psd_match_independent_parseval_oracle` [tests] `crates/whisper-adapters/src/microphone_probe.rs:962-1019`; `multitone_parseval_and_unclipped_gain_ratio_match_independent_oracle` [tests] `crates/whisper-adapters/src/microphone_probe.rs:1022-1070`; `dephased_psd_snapshot_reuses_rms_without_publishing_it_early` [tests] `crates/whisper-adapters/src/microphone_probe.rs:1073-1130`; `mixed_gain_and_limiter_counts_follow_the_rms_window` [tests] `crates/whisper-adapters/src/microphone_probe.rs:1133-1158`; `dropped_probe_blocks_force_a_full_rebuild_and_old_snapshots_expire` [tests] `crates/whisper-adapters/src/microphone_probe.rs:1161-1194`; `bounded_nonblocking_queue_counts_overflow_and_readers_hold_independent_snapshots` [tests] `crates/whisper-adapters/src/microphone_probe.rs:1197-1251`.

Déclarations `pub` (visibilité sous conditions) : `pub struct SpectralDensity {` `crates/whisper-adapters/src/microphone_probe.rs:35`; `pub struct ProbeSnapshot {` `crates/whisper-adapters/src/microphone_probe.rs:44`; `pub struct CachedSpectrum {` `crates/whisper-adapters/src/microphone_probe.rs:73`; `pub struct MicrophoneSpectrumReader {` `crates/whisper-adapters/src/microphone_probe.rs:88`; `pub fn poll(` `crates/whisper-adapters/src/microphone_probe.rs:95`; `pub struct MicrophoneProbe {` `crates/whisper-adapters/src/microphone_probe.rs:201`; `pub fn start(` `crates/whisper-adapters/src/microphone_probe.rs:219`; `pub fn submit(&self, block: ProbeAudioBlock) {` `crates/whisper-adapters/src/microphone_probe.rs:296`; `pub fn latest_at(&self, now: Instant) -> Option<Arc<ProbeSnapshot>> {` `crates/whisper-adapters/src/microphone_probe.rs:342`; `pub fn latest_spectrum_at(&self, now: Instant) -> Option<CachedSpectrum> {` `crates/whisper-adapters/src/microphone_probe.rs:358`; `pub fn epoch(&self) -> u64 {` `crates/whisper-adapters/src/microphone_probe.rs:371`; `pub fn advance_epoch(&self) -> (u64, u64) {` `crates/whisper-adapters/src/microphone_probe.rs:376`; `pub fn dropped_updates(&self) -> u64 {` `crates/whisper-adapters/src/microphone_probe.rs:391`; `pub fn invalidate(&self) {` `crates/whisper-adapters/src/microphone_probe.rs:401`; `pub fn meter(&self) -> Option<MicrophoneMeter> {` `crates/whisper-adapters/src/microphone_probe.rs:428`.

Dépend de, via `use crate::` : hors tests : `crate::capture::ProbeAudioBlock` → `crate::capture` `crates/whisper-adapters/src/microphone_probe.rs:21`<br>tests : `crate::capture::CaptureStream` → `crate::capture` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>tests : `crate::capture::Pcm16Converter` → `crate::capture` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>tests : `crate::capture` → `crate::capture` `crates/whisper-adapters/src/microphone_probe.rs:714`<br>tests : `crate::direct_live::WavOutput` → `crate::direct_live` `crates/whisper-adapters/src/microphone_probe.rs:714`.

Utilisé par, via ces imports : tests : `crate::direct_live::tests` `crates/whisper-adapters/src/direct_live/tests.rs:8`<br>tests : `crate::direct_live::tests` `crates/whisper-adapters/src/direct_live/tests.rs:8`<br>tests : `crate::direct_live::tests` `crates/whisper-adapters/src/direct_live/tests.rs:8`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>tests : `crate::parakeet_live::microphone_spectrum_tests` `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:2`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:11`.

#### Inline `tests`

Tests/auxiliaires : `gain` `crates/whisper-adapters/src/microphone_probe.rs:719-724`, `probe_and_spectrum_reader_leave_stt_pcm_and_final_wav_byte_identical` `crates/whisper-adapters/src/microphone_probe.rs:727-819`, `repeated_fresh_spectrum_snapshot_does_not_emit_unavailable` `crates/whisper-adapters/src/microphone_probe.rs:822-876`, `retry_epoch_discards_audio_acquired_before_the_retry_barrier` `crates/whisper-adapters/src/microphone_probe.rs:879-959`. `crates/whisper-adapters/src/microphone_probe.rs:711-1252`

Mesure locale : 0 production / 542 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/microphone_probe.rs:711-1252`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/microphone_probe.rs:711-1252`.

Imports locaux : `crate::capture::CaptureStream` `crates/whisper-adapters/src/microphone_probe.rs:714`; `crate::capture::Pcm16Converter` `crates/whisper-adapters/src/microphone_probe.rs:714`; `crate::capture` `crates/whisper-adapters/src/microphone_probe.rs:714`; `crate::direct_live::WavOutput` `crates/whisper-adapters/src/microphone_probe.rs:714`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::model_catalog — `crates/whisper-adapters/src/model_catalog.rs`

Inspecte les modèles locaux et met en cache les empreintes de leurs fichiers. `crates/whisper-adapters/src/model_catalog.rs:1-549`

Mesure : 455 production / 94 tests / 549 physiques. `crates/whisper-adapters/src/model_catalog.rs:1-549`

Types : `CachedModelHash` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:21-28`; `ModelFileHashCache` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:31-33`; `OpenModelFile` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:35-41`; `ArtifactFileIdentity` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:195-200`; `CatalogEntry` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:219-229`; `CatalogSnapshot` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:232-235`; `LocalModelCatalog` (struct, hors tests) `crates/whisper-adapters/src/model_catalog.rs:238-240`; `TestRoot` (struct, tests) `crates/whisper-adapters/src/model_catalog.rs:469`.

Fonctions avec corps : `open` `crates/whisper-adapters/src/model_catalog.rs:44-70`; `matches` `crates/whisper-adapters/src/model_catalog.rs:72-77`; `sha256` `crates/whisper-adapters/src/model_catalog.rs:81-112`; `full_read_count` `crates/whisper-adapters/src/model_catalog.rs:114-127`; `cached_model_sha256` `crates/whisper-adapters/src/model_catalog.rs:132-138`; `cached_artifact_sha256` `crates/whisper-adapters/src/model_catalog.rs:143-146`; `cached_artifact_sha256_with_read` `crates/whisper-adapters/src/model_catalog.rs:149-155`; `cached_file_sha256` `crates/whisper-adapters/src/model_catalog.rs:157-165`; `model_sha256_full_read_count` `crates/whisper-adapters/src/model_catalog.rs:169-175`; `artifact_sha256_full_read_count` `crates/whisper-adapters/src/model_catalog.rs:179-185`; `artifact_sha256_full_read_total` `crates/whisper-adapters/src/model_catalog.rs:189-191`; `artifact_file_identity` `crates/whisper-adapters/src/model_catalog.rs:202-210`; `model_sha256_full_read_total` `crates/whisper-adapters/src/model_catalog.rs:214-216`; `new` `crates/whisper-adapters/src/model_catalog.rs:243-245`; `inspect` `crates/whisper-adapters/src/model_catalog.rs:247-259`; `root` `crates/whisper-adapters/src/model_catalog.rs:261-263`; `engine_snapshot` `crates/whisper-adapters/src/model_catalog.rs:267-281`; `bind_turbo_import_request` `crates/whisper-adapters/src/model_catalog.rs:285-303`; `bind_turbo_live_request` `crates/whisper-adapters/src/model_catalog.rs:305-323`; `ready_turbo_path` `crates/whisper-adapters/src/model_catalog.rs:325-358`; `resolve_model_identity` `crates/whisper-adapters/src/model_catalog.rs:360-378`; `inspect_entry` `crates/whisper-adapters/src/model_catalog.rs:381-434`; `inspect_file` `crates/whisper-adapters/src/model_catalog.rs:436-442`; `decode_sha256` `crates/whisper-adapters/src/model_catalog.rs:444-454`; `new` [tests] `crates/whisper-adapters/src/model_catalog.rs:472-480`; `drop` [tests] `crates/whisper-adapters/src/model_catalog.rs:484-486`; `execution_artifact_cache_keys_hardlink_paths_and_rehashes_replaced_identity` [tests] `crates/whisper-adapters/src/model_catalog.rs:490-529`; `concurrent_execution_artifact_requests_hash_one_identity_once` [tests] `crates/whisper-adapters/src/model_catalog.rs:532-548`.

Déclarations `pub` (visibilité sous conditions) : `pub fn cached_model_sha256(path: &Path) -> io::Result<(u64, [u8; 32])> {` `crates/whisper-adapters/src/model_catalog.rs:132`; `pub fn cached_artifact_sha256(path: &Path) -> io::Result<(u64, [u8; 32])> {` `crates/whisper-adapters/src/model_catalog.rs:143`; `pub fn cached_artifact_sha256_with_read(path: &Path) -> io::Result<(u64, [u8; 32], bool)> {` `crates/whisper-adapters/src/model_catalog.rs:149`; `pub fn model_sha256_full_read_count(path: &Path) -> io::Result<u64> {` `crates/whisper-adapters/src/model_catalog.rs:169`; `pub fn artifact_sha256_full_read_count(path: &Path) -> io::Result<u64> {` `crates/whisper-adapters/src/model_catalog.rs:179`; `pub fn artifact_sha256_full_read_total() -> u64 {` `crates/whisper-adapters/src/model_catalog.rs:189`; `pub struct ArtifactFileIdentity {` `crates/whisper-adapters/src/model_catalog.rs:195`; `pub fn artifact_file_identity(path: &Path) -> io::Result<ArtifactFileIdentity> {` `crates/whisper-adapters/src/model_catalog.rs:202`; `pub fn model_sha256_full_read_total() -> u64 {` `crates/whisper-adapters/src/model_catalog.rs:214`; `pub struct CatalogEntry {` `crates/whisper-adapters/src/model_catalog.rs:219`; `pub struct CatalogSnapshot {` `crates/whisper-adapters/src/model_catalog.rs:232`; `pub struct LocalModelCatalog {` `crates/whisper-adapters/src/model_catalog.rs:238`; `pub fn new(root: impl Into<PathBuf>) -> Self {` `crates/whisper-adapters/src/model_catalog.rs:243`; `pub fn inspect(&self, generation: u64) -> CatalogSnapshot {` `crates/whisper-adapters/src/model_catalog.rs:247`; `pub fn root(&self) -> &Path {` `crates/whisper-adapters/src/model_catalog.rs:261`; `pub fn engine_snapshot(` `crates/whisper-adapters/src/model_catalog.rs:267`; `pub fn bind_turbo_import_request(` `crates/whisper-adapters/src/model_catalog.rs:285`; `pub fn bind_turbo_live_request(` `crates/whisper-adapters/src/model_catalog.rs:305`; `pub fn resolve_model_identity(` `crates/whisper-adapters/src/model_catalog.rs:360`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/model_catalog.rs:1-549`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `execution_artifact_cache_tests`

Tests/auxiliaires : `new` `crates/whisper-adapters/src/model_catalog.rs:472-480`, `drop` `crates/whisper-adapters/src/model_catalog.rs:484-486`, `execution_artifact_cache_keys_hardlink_paths_and_rehashes_replaced_identity` `crates/whisper-adapters/src/model_catalog.rs:490-529`, `concurrent_execution_artifact_requests_hash_one_identity_once` `crates/whisper-adapters/src/model_catalog.rs:532-548`. `crates/whisper-adapters/src/model_catalog.rs:456-549`

Mesure locale : 0 production / 94 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/model_catalog.rs:456-549`

Types locaux : `TestRoot` `crates/whisper-adapters/src/model_catalog.rs:469`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/model_catalog.rs:456-549`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_audio — `crates/whisper-adapters/src/parakeet_audio.rs`

Écrit les parties audio et acquittements et décode les imports en parties. `crates/whisper-adapters/src/parakeet_audio.rs:1-1061`

Mesure : 861 production / 200 tests / 1061 physiques. `crates/whisper-adapters/src/parakeet_audio.rs:1-1061`

Types : `AudioPart` (struct, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:33-38`; `AudioStoreError` (enum, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:41-47`; `CaptureWriteAck` (struct, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:50-56`; `CaptureWriterError` (enum, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:59-65`; `CaptureWriterCommand` (enum, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:67-79`; `CapturePcmWriter` (struct, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:83-89`; `PcmPartWriter` (struct, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:346-358`; `PcmPartStore` (struct, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:525-528`; `LinearPcmResampler` (struct, hors tests) `crates/whisper-adapters/src/parakeet_audio.rs:533-539`.

Fonctions avec corps : `start` `crates/whisper-adapters/src/parakeet_audio.rs:92-94`; `start_with_limit` `crates/whisper-adapters/src/parakeet_audio.rs:96-182`; `start_with_test_part_limit` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:185-191`; `try_append` `crates/whisper-adapters/src/parakeet_audio.rs:195-217`; `try_acknowledgement` `crates/whisper-adapters/src/parakeet_audio.rs:219-225`; `submitted_blocks` `crates/whisper-adapters/src/parakeet_audio.rs:227-229`; `written_samples` `crates/whisper-adapters/src/parakeet_audio.rs:231-237`; `read_written_window` `crates/whisper-adapters/src/parakeet_audio.rs:241-250`; `close` `crates/whisper-adapters/src/parakeet_audio.rs:252-294`; `drain_acknowledgements_for_close` `crates/whisper-adapters/src/parakeet_audio.rs:296-306`; `drop` `crates/whisper-adapters/src/parakeet_audio.rs:310-329`; `fmt` `crates/whisper-adapters/src/parakeet_audio.rs:333-335`; `from` `crates/whisper-adapters/src/parakeet_audio.rs:339-341`; `create` `crates/whisper-adapters/src/parakeet_audio.rs:361-363`; `create_with_limit` `crates/whisper-adapters/src/parakeet_audio.rs:365-403`; `written_samples` `crates/whisper-adapters/src/parakeet_audio.rs:405-407`; `append` `crates/whisper-adapters/src/parakeet_audio.rs:409-449`; `finish` `crates/whisper-adapters/src/parakeet_audio.rs:451-456`; `rotate` `crates/whisper-adapters/src/parakeet_audio.rs:458-477`; `finalize_part` `crates/whisper-adapters/src/parakeet_audio.rs:479-502`; `write_record` `crates/whisper-adapters/src/parakeet_audio.rs:504-506`; `write_record_values` `crates/whisper-adapters/src/parakeet_audio.rs:508-521`; `new` `crates/whisper-adapters/src/parakeet_audio.rs:542-550`; `push` `crates/whisper-adapters/src/parakeet_audio.rs:552-587`; `finish` `crates/whisper-adapters/src/parakeet_audio.rs:589-607`; `open` `crates/whisper-adapters/src/parakeet_audio.rs:611-618`; `base_path` `crates/whisper-adapters/src/parakeet_audio.rs:620-622`; `read_page` `crates/whisper-adapters/src/parakeet_audio.rs:624-656`; `total_samples` `crates/whisper-adapters/src/parakeet_audio.rs:658-676`; `read_range` `crates/whisper-adapters/src/parakeet_audio.rs:678-688`; `read_window` `crates/whisper-adapters/src/parakeet_audio.rs:691-698`; `read_bounded` `crates/whisper-adapters/src/parakeet_audio.rs:700-748`; `decode_file_to_parts` `crates/whisper-adapters/src/parakeet_audio.rs:754-759`; `decode_file_to_parts_with` `crates/whisper-adapters/src/parakeet_audio.rs:763-856`; `part_path` `crates/whisper-adapters/src/parakeet_audio.rs:858-871`; `read_u64` `crates/whisper-adapters/src/parakeet_audio.rs:873-877`; `temp_dir` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:884-890`; `capture_writer_acknowledges_written_blocks_and_reads_a_serialized_snapshot` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:893-924`; `stored_audio_is_contiguous_and_range_reads_are_bounded` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:927-955`; `linear_resampler_drains_a_short_44100_source_without_resetting_at_packet_boundary` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:958-976`; `part_rotation_preserves_every_sample_across_a_test_only_small_boundary` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:979-993`; `capture_writer_reads_every_acknowledged_sample_across_test_rotation` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:996-1027`; `failed_part_creation_and_write_keep_the_written_prefix_readable` [tests] `crates/whisper-adapters/src/parakeet_audio.rs:1030-1060`.

Déclarations `pub` (visibilité sous conditions) : `pub struct AudioPart {` `crates/whisper-adapters/src/parakeet_audio.rs:33`; `pub enum AudioStoreError {` `crates/whisper-adapters/src/parakeet_audio.rs:41`; `pub struct CaptureWriteAck {` `crates/whisper-adapters/src/parakeet_audio.rs:50`; `pub enum CaptureWriterError {` `crates/whisper-adapters/src/parakeet_audio.rs:59`; `pub struct CapturePcmWriter {` `crates/whisper-adapters/src/parakeet_audio.rs:83`; `pub fn start(base_path: impl AsRef<Path>, capacity: usize) -> Result<Self, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:92`; `pub fn try_append(&mut self, samples: &[i16]) -> Result<u64, CaptureWriterError> {` `crates/whisper-adapters/src/parakeet_audio.rs:195`; `pub fn try_acknowledgement(&self) -> Result<Option<CaptureWriteAck>, CaptureWriterError> {` `crates/whisper-adapters/src/parakeet_audio.rs:219`; `pub fn submitted_blocks(&self) -> u64 {` `crates/whisper-adapters/src/parakeet_audio.rs:227`; `pub fn written_samples(&self) -> Result<u64, CaptureWriterError> {` `crates/whisper-adapters/src/parakeet_audio.rs:231`; `pub fn read_written_window(` `crates/whisper-adapters/src/parakeet_audio.rs:241`; `pub fn close(&mut self) -> Result<u64, CaptureWriterError> {` `crates/whisper-adapters/src/parakeet_audio.rs:252`; `pub struct PcmPartWriter {` `crates/whisper-adapters/src/parakeet_audio.rs:346`; `pub fn create(base_path: impl AsRef<Path>) -> Result<Self, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:361`; `pub fn written_samples(&self) -> u64 {` `crates/whisper-adapters/src/parakeet_audio.rs:405`; `pub fn append(&mut self, mut samples: &[i16]) -> Result<std::ops::Range<u64>, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:409`; `pub fn finish(mut self) -> Result<u64, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:451`; `pub struct PcmPartStore {` `crates/whisper-adapters/src/parakeet_audio.rs:525`; `pub fn open(base_path: impl AsRef<Path>) -> Self {` `crates/whisper-adapters/src/parakeet_audio.rs:611`; `pub fn base_path(&self) -> &Path {` `crates/whisper-adapters/src/parakeet_audio.rs:620`; `pub fn read_page(&self, offset: u64, limit: usize) -> Result<Vec<AudioPart>, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:624`; `pub fn total_samples(&self) -> Result<u64, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:658`; `pub fn read_range(&self, range: std::ops::Range<u64>) -> Result<Vec<i16>, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:678`; `pub fn read_window(&self, range: std::ops::Range<u64>) -> Result<Vec<i16>, AudioStoreError> {` `crates/whisper-adapters/src/parakeet_audio.rs:691`; `pub fn decode_file_to_parts(` `crates/whisper-adapters/src/parakeet_audio.rs:754`; `pub fn decode_file_to_parts_with(` `crates/whisper-adapters/src/parakeet_audio.rs:763`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_audio.rs:1-1061`.

Utilisé par, via ces imports : hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>hors tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>hors tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>hors tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`.

#### Inline `tests`

Tests/auxiliaires : `temp_dir` `crates/whisper-adapters/src/parakeet_audio.rs:884-890`, `capture_writer_acknowledges_written_blocks_and_reads_a_serialized_snapshot` `crates/whisper-adapters/src/parakeet_audio.rs:893-924`, `stored_audio_is_contiguous_and_range_reads_are_bounded` `crates/whisper-adapters/src/parakeet_audio.rs:927-955`, `linear_resampler_drains_a_short_44100_source_without_resetting_at_packet_boundary` `crates/whisper-adapters/src/parakeet_audio.rs:958-976`. `crates/whisper-adapters/src/parakeet_audio.rs:879-1061`

Mesure locale : 0 production / 183 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/parakeet_audio.rs:879-1061`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/parakeet_audio.rs:879-1061`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/parakeet_audio.rs:879-1061`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_live::confirmation — `crates/whisper-adapters/src/parakeet_confirmation.rs`

Compare les hypothèses de mots pour produire blocs confirmés et plages restantes. `crates/whisper-adapters/src/parakeet_confirmation.rs:1-1398`

Mesure : 544 production / 854 tests / 1398 physiques. `crates/whisper-adapters/src/parakeet_confirmation.rs:1-1398`

Types : `TimedWord` (struct, hors tests) `crates/whisper-adapters/src/parakeet_confirmation.rs:16-19`; `Hypothesis` (struct, hors tests) `crates/whisper-adapters/src/parakeet_confirmation.rs:22-26`; `ConfirmedBlock` (struct, hors tests) `crates/whisper-adapters/src/parakeet_confirmation.rs:29-33`; `ConfirmationDecision` (struct, hors tests) `crates/whisper-adapters/src/parakeet_confirmation.rs:36-40`; `TrackedWord` (struct, hors tests) `crates/whisper-adapters/src/parakeet_confirmation.rs:43-50`; `Confirmation` (struct, hors tests) `crates/whisper-adapters/src/parakeet_confirmation.rs:53-63`.

Fonctions avec corps : `confirmed` `crates/whisper-adapters/src/parakeet_confirmation.rs:66-68`; `confirmed_end` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:71-73`; `unresolved` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:76-78`; `recovery_ranges` `crates/whisper-adapters/src/parakeet_confirmation.rs:80-89`; `recovery_candidates` `crates/whisper-adapters/src/parakeet_confirmation.rs:94-122`; `remaining_ranges` `crates/whisper-adapters/src/parakeet_confirmation.rs:124-140`; `has_remainder` `crates/whisper-adapters/src/parakeet_confirmation.rs:142-144`; `finalize` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:147-149`; `finalize_observed` `crates/whisper-adapters/src/parakeet_confirmation.rs:151-158`; `mark_unresolved` `crates/whisper-adapters/src/parakeet_confirmation.rs:160-168`; `ingest` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:171-174`; `ingest_observed` `crates/whisper-adapters/src/parakeet_confirmation.rs:176-329`; `publish_ready` `crates/whisper-adapters/src/parakeet_confirmation.rs:331-376`; `block_from_words` `crates/whisper-adapters/src/parakeet_confirmation.rs:378-410`; `normalized_word` `crates/whisper-adapters/src/parakeet_confirmation.rs:413-418`; `same_word` `crates/whisper-adapters/src/parakeet_confirmation.rs:420-423`; `same_span` `crates/whisper-adapters/src/parakeet_confirmation.rs:425-434`; `revision_has_context` `crates/whisper-adapters/src/parakeet_confirmation.rs:436-484`; `gap_fully_covered` `crates/whisper-adapters/src/parakeet_confirmation.rs:486-538`; `concordant_on` `crates/whisper-adapters/src/parakeet_confirmation.rs:540-548`; `words_overlapping` `crates/whisper-adapters/src/parakeet_confirmation.rs:550-556`; `ranges_overlap` `crates/whisper-adapters/src/parakeet_confirmation.rs:558-560`; `range` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:566-568`; `word` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:570-575`; `stable_prefix_progresses_and_is_immutable` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:578-607`; `invalid_time_cannot_enter_ledger` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:610-622`; `unmatched_word_blocks_later_confirmation` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:625-656`; `provisional_revision_needs_two_later_witnesses` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:659-736`; `adjacent_words_inside_time_tolerance_are_not_the_same_acoustic_word` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:739-770`; `adjacent_repeated_word_requires_its_own_temporal_witness` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:773-798`; `ambiguous_overlap_does_not_supersede_an_earlier_word` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:801-819`; `shifted_distinct_word_without_shared_context_cannot_erase_predecessor` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:822-840`; `complete_concordant_coverage_resolves_a_full_window_gap` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:843-863`; `partial_coverage_keeps_gap_until_concordant_pieces_join` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:866-896`; `empty_first_piece_cannot_borrow_a_word_from_the_second_piece` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:899-926`; `a_word_in_only_part_of_the_gap_cannot_release_later_words` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:929-952`; `an_ambiguous_interval_between_agreed_words_stays_unresolved` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:955-969`; `invalid_native_time_never_supplies_gap_evidence` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:972-993`; `unresolved_gap_preserves_prefix_and_allows_independent_later_confirmation` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:996-1034`; `later_agreed_words_publish_independently_across_an_uncovered_prefix` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1037-1058`; `two_empty_hypotheses_cannot_resolve_a_gap_before_later_text` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1061-1088`; `repeated_word_requires_two_distinct_temporal_matches` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1091-1110`; `expired_gap_does_not_block_a_stable_later_word` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1113-1139`; `confirmation_progresses_when_old_anchor_leaves_the_window` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1142-1173`; `finalization_never_promotes_a_single_hypothesis` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1176-1187`; `identical_audio_context_is_not_a_second_proof` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1190-1208`; `pending_word_is_marked_for_recovery_before_the_next_window_passes_it` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1211-1231`; `contextual_recovery_witness_confirms_without_duplication_and_keeps_prefix` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1234-1288`; `failed_contextual_recovery_keeps_word_and_gap_unresolved` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1291-1330`; `partial_contextual_recovery_does_not_resolve_the_rest_of_a_gap` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1333-1352`; `later_segment_publishes_across_an_unresolved_prefix_and_ids_survive_recovery` [tests] `crates/whisper-adapters/src/parakeet_confirmation.rs:1355-1397`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_confirmation.rs:1-1398`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_confirmation.rs:1-1398`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `range` `crates/whisper-adapters/src/parakeet_confirmation.rs:566-568`, `word` `crates/whisper-adapters/src/parakeet_confirmation.rs:570-575`, `stable_prefix_progresses_and_is_immutable` `crates/whisper-adapters/src/parakeet_confirmation.rs:578-607`, `invalid_time_cannot_enter_ledger` `crates/whisper-adapters/src/parakeet_confirmation.rs:610-622`. `crates/whisper-adapters/src/parakeet_confirmation.rs:562-1398`

Mesure locale : 0 production / 837 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/parakeet_confirmation.rs:562-1398`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/parakeet_confirmation.rs:562-1398`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/parakeet_confirmation.rs:562-1398`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_document — `crates/whisper-adapters/src/parakeet_document.rs`

Persiste segments finaux/couvertures/previews et sert les pages du document. `crates/whisper-adapters/src/parakeet_document.rs:1-1066`

Mesure : 736 production / 330 tests / 1066 physiques. `crates/whisper-adapters/src/parakeet_document.rs:1-1066`

Types : `ResumeCoverageCheckpoint` (struct, hors tests) `crates/whisper-adapters/src/parakeet_document.rs:18-22`; `ResumeCoverageReader` (struct, hors tests) `crates/whisper-adapters/src/parakeet_document.rs:24-38`; `ParakeetDocumentStore` (struct, hors tests) `crates/whisper-adapters/src/parakeet_document.rs:44-55`.

Fonctions avec corps : `create` `crates/whisper-adapters/src/parakeet_document.rs:58-93`; `revision` `crates/whisper-adapters/src/parakeet_document.rs:95-97`; `open_resume_coverage` `crates/whisper-adapters/src/parakeet_document.rs:99-106`; `latest_resume_checkpoint` `crates/whisper-adapters/src/parakeet_document.rs:108-138`; `check_identity` `crates/whisper-adapters/src/parakeet_document.rs:140-146`; `open` `crates/whisper-adapters/src/parakeet_document.rs:150-206`; `read_page` `crates/whisper-adapters/src/parakeet_document.rs:208-249`; `read_document_page` `crates/whisper-adapters/src/parakeet_document.rs:251-320`; `validate_document_remainder` `crates/whisper-adapters/src/parakeet_document.rs:322-335`; `validate_document_through` `crates/whisper-adapters/src/parakeet_document.rs:337-391`; `read_bounded_line` `crates/whisper-adapters/src/parakeet_document.rs:394-427`; `append_final` `crates/whisper-adapters/src/parakeet_document.rs:430-478`; `record_coverage` `crates/whisper-adapters/src/parakeet_document.rs:480-537`; `read_page` `crates/whisper-adapters/src/parakeet_document.rs:539-621`; `finalize` `crates/whisper-adapters/src/parakeet_document.rs:623-650`; `replace_preview` `crates/whisper-adapters/src/parakeet_document.rs:652-699`; `read_preview` `crates/whisper-adapters/src/parakeet_document.rs:701-724`; `read_u64` `crates/whisper-adapters/src/parakeet_document.rs:727-731`; `storage_error` `crates/whisper-adapters/src/parakeet_document.rs:733-735`; `segment` [tests] `crates/whisper-adapters/src/parakeet_document.rs:742-752`; `document_reads_bounded_pages_at_a_stable_revision_and_finalizes_once` [tests] `crates/whisper-adapters/src/parakeet_document.rs:755-792`; `document_preview_is_a_single_revisable_record_separate_from_final_pages` [tests] `crates/whisper-adapters/src/parakeet_document.rs:795-830`; `resume_coverage_selects_the_last_checkpoint_within_the_archived_prefix` [tests] `crates/whisper-adapters/src/parakeet_document.rs:833-889`; `resume_coverage_rejects_impossible_window_geometry_and_accepts_short_terminal_eof` [tests] `crates/whisper-adapters/src/parakeet_document.rs:892-1003`; `resume_coverage_pages_are_capped_at_128_and_keep_the_global_cursor` [tests] `crates/whisper-adapters/src/parakeet_document.rs:1006-1065`.

Déclarations `pub` (visibilité sous conditions) : `pub struct ResumeCoverageCheckpoint {` `crates/whisper-adapters/src/parakeet_document.rs:18`; `pub struct ResumeCoverageReader {` `crates/whisper-adapters/src/parakeet_document.rs:24`; `pub struct ParakeetDocumentStore {` `crates/whisper-adapters/src/parakeet_document.rs:44`; `pub fn create(` `crates/whisper-adapters/src/parakeet_document.rs:58`; `pub fn revision(&self) -> u64 {` `crates/whisper-adapters/src/parakeet_document.rs:95`; `pub fn open_resume_coverage(` `crates/whisper-adapters/src/parakeet_document.rs:99`; `pub fn latest_resume_checkpoint(` `crates/whisper-adapters/src/parakeet_document.rs:108`; `pub fn read_page(&mut self, limit: usize) -> Result<Vec<ParakeetWindowCoverage>, String> {` `crates/whisper-adapters/src/parakeet_document.rs:208`; `pub fn read_document_page(` `crates/whisper-adapters/src/parakeet_document.rs:251`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_document.rs:1-1066`.

Utilisé par, via ces imports : tests : `crate::parakeet_pipeline` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`.

#### Inline `tests`

Tests/auxiliaires : `segment` `crates/whisper-adapters/src/parakeet_document.rs:742-752`, `document_reads_bounded_pages_at_a_stable_revision_and_finalizes_once` `crates/whisper-adapters/src/parakeet_document.rs:755-792`, `document_preview_is_a_single_revisable_record_separate_from_final_pages` `crates/whisper-adapters/src/parakeet_document.rs:795-830`, `resume_coverage_selects_the_last_checkpoint_within_the_archived_prefix` `crates/whisper-adapters/src/parakeet_document.rs:833-889`. `crates/whisper-adapters/src/parakeet_document.rs:737-1066`

Mesure locale : 0 production / 330 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/parakeet_document.rs:737-1066`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/parakeet_document.rs:737-1066`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/parakeet_document.rs:737-1066`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_import::historical_runtime — `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs`

Vérifie les chemins de runtimes et références historiques de qualification. `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:1-364`

Mesure : 364 production / 0 tests / 364 physiques. `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:1-364`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:1-364`.

Fonctions avec corps : `validate_acceptance_runtime` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:5-18`; `is_shared_model` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:20-35`; `validate_gpu_qualification` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:37-66`; `validate_qualification_runtime` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:68-88`; `is_canonical_l3b_reference` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:91-139`; `is_canonical_l3c_test_artifact` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:142-152`; `is_canonical_l3c_test_artifact` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:155-157`; `is_l2b_runtime` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:159-172`; `l3_acceptance_execution_root` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:176-195`; `l3b_r6_cpu_read_runtime_root` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:200-218`; `l3b_r6_cpu_read_runtime_root_in_repository` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:220-284`; `validate_r6_read_root` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:286-293`; `historical_log_root` `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:295-364`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:1-364`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:1-364`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_import::legacy_runtime_tests — `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs`

Tests ou supports : définit `personal_cpu_and_gpu_do_not_require_l3b` `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:27-76`, `historical_cpu_requires_exact_worker_configuration_and_hash` `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:79-124`.

Mesure : 0 production / 124 tests / 124 physiques. `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:1-124`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:1-124`.

Fonctions avec corps : `fixture_root` [tests] `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:6-15`; `personal_paths` [tests] `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:17-24`; `personal_cpu_and_gpu_do_not_require_l3b` [tests] `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:27-76`; `historical_cpu_requires_exact_worker_configuration_and_hash` [tests] `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:79-124`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:1-124`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs:1-124`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_import::runtime_isolation_tests — `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs`

Tests ou supports : définit `personal_session_is_independent_of_historical_environment` `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:7-9`.

Mesure : 0 production / 9 tests / 9 physiques. `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:1-9`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:1-9`.

Fonctions avec corps : `personal_session_is_independent_of_historical_environment` [tests] `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:7-9`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:1-9`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_import/runtime_isolation_tests.rs:1-9`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_import — `crates/whisper-adapters/src/parakeet_import.rs`

Vérifie les ressources Parakeet et contrôle le processus et ses échanges IPC. `crates/whisper-adapters/src/parakeet_import.rs:1-937`

Mesure : 853 production / 84 tests / 937 physiques. `crates/whisper-adapters/src/parakeet_import.rs:1-937`

Types : `ParakeetWorkerPaths` (struct, hors tests) `crates/whisper-adapters/src/parakeet_import.rs:24-29`; `ParakeetProcess` (struct, hors tests) `crates/whisper-adapters/src/parakeet_import.rs:350-363`.

Fonctions avec corps : `validate` `crates/whisper-adapters/src/parakeet_import.rs:32-65`; `matches_personal_layout` `crates/whisper-adapters/src/parakeet_import.rs:67-98`; `is_personal_runtime` `crates/whisper-adapters/src/parakeet_import.rs:100-105`; `verify_qualification` `crates/whisper-adapters/src/parakeet_import.rs:107-125`; `qualification_key` `crates/whisper-adapters/src/parakeet_import.rs:129-171`; `gpu_qualification_key` `crates/whisper-adapters/src/parakeet_import.rs:173-205`; `personal_qualification_key` `crates/whisper-adapters/src/parakeet_import.rs:207-239`; `personal_candidate_keys_from_env` `crates/whisper-adapters/src/parakeet_import.rs:243-284`; `verify_worker_runtime` `crates/whisper-adapters/src/parakeet_import.rs:287-303`; `personal_runtime_path_is_allowed` `crates/whisper-adapters/src/parakeet_import.rs:306-310`; `runtime_sha256` `crates/whisper-adapters/src/parakeet_import.rs:312-338`; `sha256_file` `crates/whisper-adapters/src/parakeet_import.rs:340-348`; `spawn` `crates/whisper-adapters/src/parakeet_import.rs:366-368`; `spawn_live` `crates/whisper-adapters/src/parakeet_import.rs:371-381`; `spawn_for_qualification` `crates/whisper-adapters/src/parakeet_import.rs:385-394`; `spawn_profile` `crates/whisper-adapters/src/parakeet_import.rs:396-398`; `spawn_profile_with_log_root` `crates/whisper-adapters/src/parakeet_import.rs:400-518`; `poll` `crates/whisper-adapters/src/parakeet_import.rs:520-538`; `transcribe` `crates/whisper-adapters/src/parakeet_import.rs:540-556`; `transcribe_with_gpu` `crates/whisper-adapters/src/parakeet_import.rs:558-576`; `transcribe_with_gpu_correlated` `crates/whisper-adapters/src/parakeet_import.rs:579-608`; `accept_ready` `crates/whisper-adapters/src/parakeet_import.rs:610-616`; `identity` `crates/whisper-adapters/src/parakeet_import.rs:618-620`; `pid` `crates/whisper-adapters/src/parakeet_import.rs:622-624`; `inject_incomplete_result_for_acceptance` `crates/whisper-adapters/src/parakeet_import.rs:627-638`; `shutdown` `crates/whisper-adapters/src/parakeet_import.rs:640-642`; `terminate` `crates/whisper-adapters/src/parakeet_import.rs:644-658`; `try_wait` `crates/whisper-adapters/src/parakeet_import.rs:660-664`; `write` `crates/whisper-adapters/src/parakeet_import.rs:666-676`; `worker_stderr_log` `crates/whisper-adapters/src/parakeet_import.rs:679-737`; `worker_log_prefix` `crates/whisper-adapters/src/parakeet_import.rs:739-749`; `gpu_log_prefix_preserves_l3b_and_scopes_l3c_names` [tests] `crates/whisper-adapters/src/parakeet_import.rs:754-768`; `expected_gpu_for_candidate` `crates/whisper-adapters/src/parakeet_import.rs:778-844`; `normalize_pci_bus_id` `crates/whisper-adapters/src/parakeet_import.rs:846-857`; `decode_worker_frame` `crates/whisper-adapters/src/parakeet_import.rs:859-867`; `drop` `crates/whisper-adapters/src/parakeet_import.rs:870-875`; `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/src/parakeet_import.rs:882-896`; `paths_reject_a_worker_outside_the_runtime_root` [tests] `crates/whisper-adapters/src/parakeet_import.rs:899-918`; `personal_runtime_root_is_published_or_direct_staging_child_only` [tests] `crates/whisper-adapters/src/parakeet_import.rs:921-936`.

Déclarations `pub` (visibilité sous conditions) : `pub struct ParakeetWorkerPaths {` `crates/whisper-adapters/src/parakeet_import.rs:24`; `pub fn validate(&self) -> Result<(), String> {` `crates/whisper-adapters/src/parakeet_import.rs:32`; `pub fn verify_qualification(` `crates/whisper-adapters/src/parakeet_import.rs:107`; `pub fn qualification_key(` `crates/whisper-adapters/src/parakeet_import.rs:129`; `pub fn gpu_qualification_key(` `crates/whisper-adapters/src/parakeet_import.rs:173`; `pub fn personal_candidate_keys_from_env() -> Vec<crate::archive::ArchiveQualificationKey> {` `crates/whisper-adapters/src/parakeet_import.rs:243`; `pub fn verify_worker_runtime(` `crates/whisper-adapters/src/parakeet_import.rs:287`; `pub struct ParakeetProcess {` `crates/whisper-adapters/src/parakeet_import.rs:350`; `pub fn spawn(paths: &ParakeetWorkerPaths) -> Result<Self, String> {` `crates/whisper-adapters/src/parakeet_import.rs:366`; `pub fn spawn_for_qualification(` `crates/whisper-adapters/src/parakeet_import.rs:385`; `pub fn poll(&mut self) -> Result<Option<Event>, String> {` `crates/whisper-adapters/src/parakeet_import.rs:520`; `pub fn transcribe(` `crates/whisper-adapters/src/parakeet_import.rs:540`; `pub fn transcribe_with_gpu(` `crates/whisper-adapters/src/parakeet_import.rs:558`; `pub fn accept_ready(&mut self, identity: WorkerIdentity) -> Result<(), String> {` `crates/whisper-adapters/src/parakeet_import.rs:610`; `pub fn identity(&self) -> Option<&WorkerIdentity> {` `crates/whisper-adapters/src/parakeet_import.rs:618`; `pub fn pid(&self) -> u32 {` `crates/whisper-adapters/src/parakeet_import.rs:622`; `pub fn shutdown(&mut self) -> Result<(), String> {` `crates/whisper-adapters/src/parakeet_import.rs:640`; `pub fn terminate(&mut self) -> Result<std::process::ExitStatus, String> {` `crates/whisper-adapters/src/parakeet_import.rs:644`; `pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {` `crates/whisper-adapters/src/parakeet_import.rs:660`; `pub fn expected_gpu_for_candidate(` `crates/whisper-adapters/src/parakeet_import.rs:778`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_import.rs:1-937`.

Utilisé par, via ces imports : tests : `crate::direct_live::d2_corpus` `crates/whisper-adapters/src/direct_live/d2_corpus.rs:1`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>tests : `crate::parakeet_import::runtime_isolation_tests / integration::parakeet_runtime_isolation / integration::support::parakeet_runtime_isolation` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1`<br>tests : `crate::parakeet_import::runtime_isolation_tests / integration::parakeet_runtime_isolation / integration::support::parakeet_runtime_isolation` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1`<br>tests : `crate::parakeet_import::runtime_isolation_tests / integration::parakeet_runtime_isolation / integration::support::parakeet_runtime_isolation` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1`.

#### Inline `worker_log_mode_tests`

Tests/auxiliaires : `gpu_log_prefix_preserves_l3b_and_scopes_l3c_names` `crates/whisper-adapters/src/parakeet_import.rs:754-768`. `crates/whisper-adapters/src/parakeet_import.rs:751-769`

Mesure locale : 0 production / 19 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/parakeet_import.rs:751-769`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/parakeet_import.rs:751-769`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/parakeet_import.rs:751-769`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

#### Inline `tests`

Tests/auxiliaires : `not_run_without_campaign_variables` `crates/whisper-adapters/src/parakeet_import.rs:882-896`, `paths_reject_a_worker_outside_the_runtime_root` `crates/whisper-adapters/src/parakeet_import.rs:899-918`, `personal_runtime_root_is_published_or_direct_staging_child_only` `crates/whisper-adapters/src/parakeet_import.rs:921-936`. `crates/whisper-adapters/src/parakeet_import.rs:878-937`

Mesure locale : 0 production / 60 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/parakeet_import.rs:878-937`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/parakeet_import.rs:878-937`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/parakeet_import.rs:878-937`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::parakeet_live::capture — `crates/whisper-adapters/src/parakeet_live/capture.rs`

Gère capture, gain, parties audio et acquittements du micro Parakeet. `crates/whisper-adapters/src/parakeet_live/capture.rs:1-455`

Mesure : 432 production / 23 tests / 455 physiques. `crates/whisper-adapters/src/parakeet_live/capture.rs:1-455`

Types : `UnifiedCapturePump` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live/capture.rs:6-19`; `AppliedCaptureGain` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live/capture.rs:21-26`.

Fonctions avec corps : `from_capture` `crates/whisper-adapters/src/parakeet_live/capture.rs:29-34`; `from_capture_with_test_part_limit` [tests] `crates/whisper-adapters/src/parakeet_live/capture.rs:37-43`; `from_capture_inner` `crates/whisper-adapters/src/parakeet_live/capture.rs:45-96`; `enable_probe` `crates/whisper-adapters/src/parakeet_live/capture.rs:98-111`; `take_next_probe_telemetry` `crates/whisper-adapters/src/parakeet_live/capture.rs:113-138`; `pump` `crates/whisper-adapters/src/parakeet_live/capture.rs:139-182`; `set_gain` `crates/whisper-adapters/src/parakeet_live/capture.rs:184-195`; `enqueue_test_block` [tests] `crates/whisper-adapters/src/parakeet_live/capture.rs:198-203`; `take_gain_events` `crates/whisper-adapters/src/parakeet_live/capture.rs:205-207`; `record_gain_acknowledgement` `crates/whisper-adapters/src/parakeet_live/capture.rs:209-228`; `stop` `crates/whisper-adapters/src/parakeet_live/capture.rs:230-237`; `stop_inner` `crates/whisper-adapters/src/parakeet_live/capture.rs:239-281`; `collect_available_acknowledgements` `crates/whisper-adapters/src/parakeet_live/capture.rs:283-296`; `collect_acknowledgements_until` `crates/whisper-adapters/src/parakeet_live/capture.rs:298-318`; `append_during_stop` `crates/whisper-adapters/src/parakeet_live/capture.rs:320-336`; `read_window` `crates/whisper-adapters/src/parakeet_live/capture.rs:340-346`; `pump_capture` `crates/whisper-adapters/src/parakeet_live/capture.rs:350-413`; `set_gain` `crates/whisper-adapters/src/parakeet_live/capture.rs:415-426`; `take_next_probe_telemetry` `crates/whisper-adapters/src/parakeet_live/capture.rs:428-438`; `take_probe_meter` `crates/whisper-adapters/src/parakeet_live/capture.rs:440-448`; `take_probe_spectrum` `crates/whisper-adapters/src/parakeet_live/capture.rs:450-454`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/capture.rs:1-455`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/capture.rs:1-455`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::diagnostics_absence_tests — `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs`

Tests ou supports : définit `legacy_environment_does_not_create_diagnostic_files` `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:8-56`, `d2_test_diagnostics_remain_available_in_guarded_destination` `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:59-94`.

Mesure : 0 production / 94 tests / 94 physiques. `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:1-94`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:1-94`.

Fonctions avec corps : `legacy_environment_does_not_create_diagnostic_files` [tests] `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:8-56`; `d2_test_diagnostics_remain_available_in_guarded_destination` [tests] `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:59-94`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:1-94`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/diagnostics_absence_tests.rs:1-94`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::events — `crates/whisper-adapters/src/parakeet_live/events.rs`

Draine les résultats de fenêtres et produit les événements progression/audio. `crates/whisper-adapters/src/parakeet_live/events.rs:1-280`

Mesure : 238 production / 42 tests / 280 physiques. `crates/whisper-adapters/src/parakeet_live/events.rs:1-280`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_live/events.rs:1-280`.

Fonctions avec corps : `poll` `crates/whisper-adapters/src/parakeet_live/events.rs:4-237`; `remove_completed_window` `crates/whisper-adapters/src/parakeet_live/events.rs:239-241`; `queue_progress` `crates/whisper-adapters/src/parakeet_live/events.rs:243-249`; `queue_audio_unusable` `crates/whisper-adapters/src/parakeet_live/events.rs:251-274`; `pop_pending_event` `crates/whisper-adapters/src/parakeet_live/events.rs:276-279`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/events.rs:1-280`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/events.rs:1-280`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::inference — `crates/whisper-adapters/src/parakeet_live/inference.rs`

Exécute les inférences Parakeet et écrit les WAV de fenêtre. `crates/whisper-adapters/src/parakeet_live/inference.rs:1-531`

Mesure : 531 production / 0 tests / 531 physiques. `crates/whisper-adapters/src/parakeet_live/inference.rs:1-531`

Types : `InferenceThreadContext` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live/inference.rs:10-15`; `WindowRequestContext` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live/inference.rs:368-374`.

Fonctions avec corps : `expected_live_worker_backend` `crates/whisper-adapters/src/parakeet_live/inference.rs:3-8`; `inference_thread` `crates/whisper-adapters/src/parakeet_live/inference.rs:17-366`; `transcribe_window` `crates/whisper-adapters/src/parakeet_live/inference.rs:376-498`; `write_pcm16_wav` `crates/whisper-adapters/src/parakeet_live/inference.rs:500-531`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/inference.rs:1-531`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/inference.rs:1-531`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::lifecycle — `crates/whisper-adapters/src/parakeet_live/lifecycle.rs`

Démarre, arrête et draine la capture et expose l’import final automatique. `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:1-452`

Mesure : 430 production / 22 tests / 452 physiques. `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:1-452`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:1-452`.

Fonctions avec corps : `start` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:4-11`; `start_test_capture` [tests] `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:14-21`; `start_with_capture` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:23-158`; `identity` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:160-162`; `enqueue_d2_capture_block` [tests] `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:165-170`; `automatic_final_import` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:172-197`; `stop` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:199-353`; `drain_stopped_capture` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:355-372`; `stop_after_capture_error` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:374-428`; `confirmed_segments` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:430-443`; `drop` `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:447-451`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:1-452`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:1-452`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::microphone_spectrum_tests — `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs`

Tests ou supports : définit `parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav` `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:11-216`.

Mesure : 0 production / 216 tests / 216 physiques. `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:1-216`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:1-216`.

Fonctions avec corps : `parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav` [tests] `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:11-216`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:1-216`.

Dépend de, via `use crate::` : tests : `crate::microphone_probe::MicrophoneSpectrumReader` → `crate::microphone_probe` `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::recovery_tests — `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs`

Tests ou supports : définit `unified_capture_recycles_slots_and_preserves_audio_beyond_ring_capacity` `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:4-65`, `unified_capture_pump_converts_synthetic_input_and_closes_the_complete_source` `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:68-96`.

Mesure : 0 production / 139 tests / 139 physiques. `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:1-139`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:1-139`.

Fonctions avec corps : `unified_capture_recycles_slots_and_preserves_audio_beyond_ring_capacity` [tests] `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:4-65`; `unified_capture_pump_converts_synthetic_input_and_closes_the_complete_source` [tests] `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:68-96`; `stop_recovery_skips_a_portion_already_attempted_during_capture` [tests] `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:99-114`; `recovery_window_keeps_context_for_the_observed_ah_word` [tests] `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:117-125`; `recovery_window_refuses_a_portion_without_durable_context` [tests] `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:128-139`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:1-139`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/recovery_tests.rs:1-139`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::trace — `crates/whisper-adapters/src/parakeet_live/trace.rs`

Enregistre les traces conditionnelles du micro Parakeet. `crates/whisper-adapters/src/parakeet_live/trace.rs:1-65`

Mesure : 19 production / 46 tests / 65 physiques. `crates/whisper-adapters/src/parakeet_live/trace.rs:1-65`

Types : `D2WorkerDiagnostics` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live/trace.rs:6-11`.

Fonctions avec corps : `default` [tests] `crates/whisper-adapters/src/parakeet_live/trace.rs:15-20`; `for_d2_destination` [tests] `crates/whisper-adapters/src/parakeet_live/trace.rs:25-41`; `enabled` `crates/whisper-adapters/src/parakeet_live/trace.rs:43-50`; `record` `crates/whisper-adapters/src/parakeet_live/trace.rs:52-64`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/trace.rs:1-65`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/trace.rs:1-65`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::window_result_contract_tests — `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs`

Tests ou supports : définit `correlated_cpu_result_does_not_require_native_call_marker` `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:27-31`, `expected_gpu_result_requires_native_call_marker` `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:34-39`.

Mesure : 0 production / 57 tests / 57 physiques. `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:1-57`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:1-57`.

Fonctions avec corps : `worker_identity` [tests] `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:3-10`; `result_identity` [tests] `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:12-24`; `correlated_cpu_result_does_not_require_native_call_marker` [tests] `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:27-31`; `expected_gpu_result_requires_native_call_marker` [tests] `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:34-39`; `foreign_result_is_rejected_even_without_gpu_marker_requirement` [tests] `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:42-50`; `configured_compute_selects_expected_ready_backend` [tests] `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:53-57`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:1-57`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/window_result_contract_tests.rs:1-57`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live::windows — `crates/whisper-adapters/src/parakeet_live/windows.rs`

Planifie et soumet les fenêtres live puis vérifie identité et mots retournés. `crates/whisper-adapters/src/parakeet_live/windows.rs:1-252`

Mesure : 252 production / 0 tests / 252 physiques. `crates/whisper-adapters/src/parakeet_live/windows.rs:1-252`

Types : `RecoveryAttemptLedger` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live/windows.rs:4-6`.

Fonctions avec corps : `len` `crates/whisper-adapters/src/parakeet_live/windows.rs:9-11`; `contains_overlap` `crates/whisper-adapters/src/parakeet_live/windows.rs:13-17`; `record` `crates/whisper-adapters/src/parakeet_live/windows.rs:19-23`; `schedule_window` `crates/whisper-adapters/src/parakeet_live/windows.rs:27-109`; `submit_window` `crates/whisper-adapters/src/parakeet_live/windows.rs:111-179`; `recovery_window` `crates/whisper-adapters/src/parakeet_live/windows.rs:182-210`; `window_result_matches` `crates/whisper-adapters/src/parakeet_live/windows.rs:212-228`; `validate_words` `crates/whisper-adapters/src/parakeet_live/windows.rs:230-252`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live/windows.rs:1-252`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/parakeet_live/windows.rs:1-252`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_live — `crates/whisper-adapters/src/parakeet_live.rs`

Définit l’état et les canaux du micro Parakeet et déclare ses sous-modules. `crates/whisper-adapters/src/parakeet_live.rs:1-163`

Mesure : 151 production / 12 tests / 163 physiques. `crates/whisper-adapters/src/parakeet_live.rs:1-163`

Types : `InferenceRequest` (enum, hors tests) `crates/whisper-adapters/src/parakeet_live.rs:69-80`; `InferenceEvent` (enum, hors tests) `crates/whisper-adapters/src/parakeet_live.rs:83-106`; `WindowTranscription` (enum, hors tests) `crates/whisper-adapters/src/parakeet_live.rs:108-111`; `ParakeetLive` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live.rs:113-144`; `WindowResultIdentity` (struct, hors tests) `crates/whisper-adapters/src/parakeet_live.rs:147-155`.

Fonctions avec corps : `unix_time_ms` `crates/whisper-adapters/src/parakeet_live.rs:157-163`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/parakeet_live.rs:1-163`.

Dépend de, via `use crate::` : hors tests : `crate::microphone_probe::MicrophoneProbe` → `crate::microphone_probe` `crates/whisper-adapters/src/parakeet_live.rs:11`<br>hors tests : `crate::capture::CaptureStream` → `crate::capture` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::capture::Pcm16Converter` → `crate::capture` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::capture` → `crate::capture` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::direct_live::WavOutput` → `crate::direct_live` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_audio::CapturePcmWriter` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_audio::CaptureWriteAck` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_audio` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_import::ParakeetProcess` → `crate::parakeet_import` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_import::ParakeetWorkerPaths` → `crate::parakeet_import` `crates/whisper-adapters/src/parakeet_live.rs:12`<br>hors tests : `crate::parakeet_import` → `crate::parakeet_import` `crates/whisper-adapters/src/parakeet_live.rs:12`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_pipeline — `crates/whisper-adapters/src/parakeet_pipeline.rs`

Prépare et engage les fenêtres finales/previews à partir d’une source PCM. `crates/whisper-adapters/src/parakeet_pipeline.rs:1-1077`

Mesure : 614 production / 463 tests / 1077 physiques. `crates/whisper-adapters/src/parakeet_pipeline.rs:1-1077`

Types : `WindowTranscriber` (trait, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:14-49`; `ReadablePcmSource` (trait, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:51-53`; `PipelineResult` (struct, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:75-80`; `ParakeetResumeCheckpoint` (struct, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:83-88`; `FinalizationStep` (enum, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:91-101`; `PreparedFinalWindow` (enum, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:104-108`; `PreparedPreviewWindow` (struct, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:111-114`; `PendingFinalWindow` (struct, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:117-120`; `ParakeetFinalizer` (struct, hors tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:122-131`; `ControlledWorker` (struct, tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:628`; `BoundaryWorker` (struct, tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:647`; `SuffixWorker` (struct, tests) `crates/whisper-adapters/src/parakeet_pipeline.rs:664`.

Fonctions avec corps : `initialize_instance` `crates/whisper-adapters/src/parakeet_pipeline.rs:19-25`; `stop_instance` `crates/whisper-adapters/src/parakeet_pipeline.rs:29-31`; `transcribe_scheduled` `crates/whisper-adapters/src/parakeet_pipeline.rs:39-48`; `read_window` `crates/whisper-adapters/src/parakeet_pipeline.rs:56-58`; `read_window` `crates/whisper-adapters/src/parakeet_pipeline.rs:62-65`; `read_window` `crates/whisper-adapters/src/parakeet_pipeline.rs:69-71`; `new` `crates/whisper-adapters/src/parakeet_pipeline.rs:134-149`; `from_checkpoint` `crates/whisper-adapters/src/parakeet_pipeline.rs:151-197`; `processed_samples` `crates/whisper-adapters/src/parakeet_pipeline.rs:199-201`; `next_window_sequence` `crates/whisper-adapters/src/parakeet_pipeline.rs:203-205`; `next_segment_sequence` `crates/whisper-adapters/src/parakeet_pipeline.rs:207-209`; `prepare_next` `crates/whisper-adapters/src/parakeet_pipeline.rs:213-282`; `commit_prepared` `crates/whisper-adapters/src/parakeet_pipeline.rs:284-295`; `finish_document` `crates/whisper-adapters/src/parakeet_pipeline.rs:297-330`; `commit_window` `crates/whisper-adapters/src/parakeet_pipeline.rs:332-388`; `step` `crates/whisper-adapters/src/parakeet_pipeline.rs:393-417`; `finalize_source` `crates/whisper-adapters/src/parakeet_pipeline.rs:420-437`; `transcribe_preview` `crates/whisper-adapters/src/parakeet_pipeline.rs:441-467`; `commit_preview` `crates/whisper-adapters/src/parakeet_pipeline.rs:469-490`; `prepare_preview` `crates/whisper-adapters/src/parakeet_pipeline.rs:492-512`; `validated_segments` `crates/whisper-adapters/src/parakeet_pipeline.rs:514-606`; `store_error` `crates/whisper-adapters/src/parakeet_pipeline.rs:608-610`; `port_error` `crates/whisper-adapters/src/parakeet_pipeline.rs:611-613`; `instance_id` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:630-632`; `transcribe` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:633-644`; `instance_id` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:650-652`; `transcribe` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:654-661`; `instance_id` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:667-669`; `transcribe` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:671-682`; `word` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:685-691`; `only_the_last_word_of_the_last_window_gets_the_bounded_eof_tolerance` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:694-754`; `controlled_worker_processes_a_long_source_in_contiguous_windows` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:757-785`; `restored_finalizer_keeps_prefix_and_accepts_midpoint_owned_boundary_word` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:788-877`; `finalizer_rejects_a_checkpoint_whose_anchor_is_not_covered` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:880-903`; `resume_preserves_prefix_while_backend_suffix_text_can_differ` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:906-980`; `finalization_advances_from_acknowledged_capture_then_closes_the_tail` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:983-1062`; `wait_for_ack` [tests] `crates/whisper-adapters/src/parakeet_pipeline.rs:1064-1076`.

Déclarations `pub` (visibilité sous conditions) : `pub trait WindowTranscriber: Send {` `crates/whisper-adapters/src/parakeet_pipeline.rs:14`; `pub trait ReadablePcmSource {` `crates/whisper-adapters/src/parakeet_pipeline.rs:51`; `pub struct PipelineResult {` `crates/whisper-adapters/src/parakeet_pipeline.rs:75`; `pub struct ParakeetResumeCheckpoint {` `crates/whisper-adapters/src/parakeet_pipeline.rs:83`; `pub enum FinalizationStep {` `crates/whisper-adapters/src/parakeet_pipeline.rs:91`; `pub enum PreparedFinalWindow {` `crates/whisper-adapters/src/parakeet_pipeline.rs:104`; `pub struct PreparedPreviewWindow {` `crates/whisper-adapters/src/parakeet_pipeline.rs:111`; `pub struct ParakeetFinalizer {` `crates/whisper-adapters/src/parakeet_pipeline.rs:122`; `pub fn new(job_id: JobId, generation: Generation) -> Self {` `crates/whisper-adapters/src/parakeet_pipeline.rs:134`; `pub fn from_checkpoint(` `crates/whisper-adapters/src/parakeet_pipeline.rs:151`; `pub fn processed_samples(&self) -> u64 {` `crates/whisper-adapters/src/parakeet_pipeline.rs:199`; `pub fn next_window_sequence(&self) -> u64 {` `crates/whisper-adapters/src/parakeet_pipeline.rs:203`; `pub fn next_segment_sequence(&self) -> u64 {` `crates/whisper-adapters/src/parakeet_pipeline.rs:207`; `pub fn prepare_next<S: ReadablePcmSource + ?Sized>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:213`; `pub fn commit_prepared<D: ParakeetDocumentPort>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:284`; `pub fn finish_document<D: ParakeetDocumentPort>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:297`; `pub fn step<` `crates/whisper-adapters/src/parakeet_pipeline.rs:393`; `pub fn finalize_source<T: WindowTranscriber + ?Sized, D: ParakeetDocumentPort>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:420`; `pub fn transcribe_preview<S, T, D>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:441`; `pub fn commit_preview<D: ParakeetDocumentPort>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:469`; `pub fn prepare_preview<S: ReadablePcmSource + ?Sized>(` `crates/whisper-adapters/src/parakeet_pipeline.rs:492`.

Dépend de, via `use crate::` : hors tests : `crate::parakeet_audio::AudioStoreError` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>hors tests : `crate::parakeet_audio::CapturePcmWriter` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>hors tests : `crate::parakeet_audio::PcmPartStore` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>hors tests : `crate::parakeet_audio` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:10`<br>tests : `crate::parakeet_audio::CapturePcmWriter` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_audio::PcmPartStore` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_audio::PcmPartWriter` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_audio` → `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`<br>tests : `crate::parakeet_document::ParakeetDocumentStore` → `crate::parakeet_document` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `instance_id` `crates/whisper-adapters/src/parakeet_pipeline.rs:630-632`, `transcribe` `crates/whisper-adapters/src/parakeet_pipeline.rs:633-644`, `instance_id` `crates/whisper-adapters/src/parakeet_pipeline.rs:650-652`, `transcribe` `crates/whisper-adapters/src/parakeet_pipeline.rs:654-661`. `crates/whisper-adapters/src/parakeet_pipeline.rs:615-1077`

Mesure locale : 0 production / 463 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/parakeet_pipeline.rs:615-1077`

Types locaux : `ControlledWorker` `crates/whisper-adapters/src/parakeet_pipeline.rs:628`, `BoundaryWorker` `crates/whisper-adapters/src/parakeet_pipeline.rs:647`, `SuffixWorker` `crates/whisper-adapters/src/parakeet_pipeline.rs:664`.

Imports locaux : `crate::parakeet_audio::CapturePcmWriter` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`; `crate::parakeet_audio::PcmPartStore` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`; `crate::parakeet_audio::PcmPartWriter` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`; `crate::parakeet_audio` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`; `crate::parakeet_document::ParakeetDocumentStore` `crates/whisper-adapters/src/parakeet_pipeline.rs:618`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::path_settings — `crates/whisper-adapters/src/path_settings.rs`

Lit/écrit les préférences JSON et ouvre les sélecteurs Windows. `crates/whisper-adapters/src/path_settings.rs:1-551`

Mesure : 488 production / 63 tests / 551 physiques. `crates/whisper-adapters/src/path_settings.rs:1-551`

Types : `WindowsPathPort` (struct, hors tests) `crates/whisper-adapters/src/path_settings.rs:19-25`; `WindowsModelPreferencePort` (struct, hors tests) `crates/whisper-adapters/src/path_settings.rs:160-163`; `MemoryPathPort` (struct, tests) `crates/whisper-adapters/src/path_settings.rs:496`.

Fonctions avec corps : `new` `crates/whisper-adapters/src/path_settings.rs:28-35`; `new_at` `crates/whisper-adapters/src/path_settings.rs:37-54`; `settings_path` `crates/whisper-adapters/src/path_settings.rs:56-58`; `default` `crates/whisper-adapters/src/path_settings.rs:62-64`; `drop` `crates/whisper-adapters/src/path_settings.rs:68-73`; `load` `crates/whisper-adapters/src/path_settings.rs:77-89`; `save` `crates/whisper-adapters/src/path_settings.rs:91-102`; `pick` `crates/whisper-adapters/src/path_settings.rs:104-124`; `exists` `crates/whisper-adapters/src/path_settings.rs:126-143`; `has_pending_save` `crates/whisper-adapters/src/path_settings.rs:145-147`; `poll_save_acknowledgement` `crates/whisper-adapters/src/path_settings.rs:149-157`; `default` `crates/whisper-adapters/src/path_settings.rs:166-168`; `new` `crates/whisper-adapters/src/path_settings.rs:172-188`; `new_at` `crates/whisper-adapters/src/path_settings.rs:190-195`; `load` `crates/whisper-adapters/src/path_settings.rs:199-275`; `save` `crates/whisper-adapters/src/path_settings.rs:277-291`; `write_settings` `crates/whisper-adapters/src/path_settings.rs:294-331`; `write_model_preferences` `crates/whisper-adapters/src/path_settings.rs:333-415`; `settings_write_lock` `crates/whisper-adapters/src/path_settings.rs:417-420`; `ensure_v1_backup` `crates/whisper-adapters/src/path_settings.rs:422-452`; `read_settings_value` `crates/whisper-adapters/src/path_settings.rs:454-462`; `write_settings_value` `crates/whisper-adapters/src/path_settings.rs:464-487`; `load` [tests] `crates/whisper-adapters/src/path_settings.rs:499-501`; `save` [tests] `crates/whisper-adapters/src/path_settings.rs:502-505`; `pick` [tests] `crates/whisper-adapters/src/path_settings.rs:506-508`; `exists` [tests] `crates/whisper-adapters/src/path_settings.rs:509-511`; `has_pending_save` [tests] `crates/whisper-adapters/src/path_settings.rs:512-514`; `poll_save_acknowledgement` [tests] `crates/whisper-adapters/src/path_settings.rs:515-517`; `malformed_json_gain_isolated_while_valid_settings_and_other_gain_restore` [tests] `crates/whisper-adapters/src/path_settings.rs:521-550`.

Déclarations `pub` (visibilité sous conditions) : `pub struct WindowsPathPort {` `crates/whisper-adapters/src/path_settings.rs:19`; `pub fn new() -> Self {` `crates/whisper-adapters/src/path_settings.rs:28`; `pub fn new_at(file: PathBuf) -> Self {` `crates/whisper-adapters/src/path_settings.rs:37`; `pub fn settings_path(&self) -> &Path {` `crates/whisper-adapters/src/path_settings.rs:56`; `pub struct WindowsModelPreferencePort {` `crates/whisper-adapters/src/path_settings.rs:160`; `pub fn new() -> Self {` `crates/whisper-adapters/src/path_settings.rs:172`; `pub fn new_at(file: PathBuf, known_historical_turbo_path: impl Into<String>) -> Self {` `crates/whisper-adapters/src/path_settings.rs:190`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/path_settings.rs:1-551`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `load` `crates/whisper-adapters/src/path_settings.rs:499-501`, `save` `crates/whisper-adapters/src/path_settings.rs:502-505`, `pick` `crates/whisper-adapters/src/path_settings.rs:506-508`, `exists` `crates/whisper-adapters/src/path_settings.rs:509-511`. `crates/whisper-adapters/src/path_settings.rs:489-551`

Mesure locale : 0 production / 63 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/path_settings.rs:489-551`

Types locaux : `MemoryPathPort` `crates/whisper-adapters/src/path_settings.rs:496`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/path_settings.rs:489-551`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::queue_store — `crates/whisper-adapters/src/queue_store.rs`

Persiste/rejoue la file et valide les demandes de reprise. `crates/whisper-adapters/src/queue_store.rs:1-805`

Mesure : 598 production / 207 tests / 805 physiques. `crates/whisper-adapters/src/queue_store.rs:1-805`

Types : `QueueRecord` (enum, hors tests) `crates/whisper-adapters/src/queue_store.rs:30-40`; `QueueRecordWire` (struct, hors tests) `crates/whisper-adapters/src/queue_store.rs:43-52`; `QueueStore` (struct, hors tests) `crates/whisper-adapters/src/queue_store.rs:114-118`.

Fonctions avec corps : `queue_schema` `crates/whisper-adapters/src/queue_store.rs:14-21`; `valid_queue_upsert` `crates/whisper-adapters/src/queue_store.rs:23-25`; `serialize` `crates/whisper-adapters/src/queue_store.rs:55-80`; `deserialize` `crates/whisper-adapters/src/queue_store.rs:84-110`; `inspect_read_only` `crates/whisper-adapters/src/queue_store.rs:124-169`; `open` `crates/whisper-adapters/src/queue_store.rs:171-182`; `entries` `crates/whisper-adapters/src/queue_store.rs:184-186`; `resolve_queued_with_catalog` `crates/whisper-adapters/src/queue_store.rs:189-217`; `enqueue` `crates/whisper-adapters/src/queue_store.rs:219-246`; `set_status` `crates/whisper-adapters/src/queue_store.rs:248-267`; `set_pending_compute` `crates/whisper-adapters/src/queue_store.rs:272-314`; `resume_interrupted` `crates/whisper-adapters/src/queue_store.rs:316-340`; `validate_resume` `crates/whisper-adapters/src/queue_store.rs:342-385`; `validate_resume_with_catalog` `crates/whisper-adapters/src/queue_store.rs:390-422`; `resume_interrupted_with_catalog` `crates/whisper-adapters/src/queue_store.rs:424-450`; `remove` `crates/whisper-adapters/src/queue_store.rs:452-502`; `append` `crates/whisper-adapters/src/queue_store.rs:504-516`; `replay` `crates/whisper-adapters/src/queue_store.rs:518-558`; `root` [tests] `crates/whisper-adapters/src/queue_store.rs:566-574`; `request` [tests] `crates/whisper-adapters/src/queue_store.rs:576-592`; `pending_compute_change_persists_only_the_choice_and_keeps_fifo` [tests] `crates/whisper-adapters/src/queue_store.rs:595-623`; `pending_compute_change_rejects_identity_or_status_changes` [tests] `crates/whisper-adapters/src/queue_store.rs:626-651`; `pending_compute_change_cannot_skip_the_fifo_head` [tests] `crates/whisper-adapters/src/queue_store.rs:654-670`; `recovery_inspection_is_read_only_for_missing_and_corrupt_queues` [tests] `crates/whisper-adapters/src/queue_store.rs:673-694`; `unified_queue_schema_round_trips_strategy_and_rejects_legacy_schema_mismatch` [tests] `crates/whisper-adapters/src/queue_store.rs:697-736`; `historical_queue_json_without_strategy_reads_as_legacy_without_rewriting` [tests] `crates/whisper-adapters/src/queue_store.rs:739-766`; `repair_uncommitted_tail` `crates/whisper-adapters/src/queue_store.rs:769-805`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};` `crates/whisper-adapters/src/queue_store.rs:27`; `pub struct QueueStore {` `crates/whisper-adapters/src/queue_store.rs:114`; `pub fn inspect_read_only(destination: impl AsRef<Path>) -> Result<Vec<QueueEntry>, String> {` `crates/whisper-adapters/src/queue_store.rs:124`; `pub fn open(destination: impl AsRef<Path>) -> Result<Self, String> {` `crates/whisper-adapters/src/queue_store.rs:171`; `pub fn entries(&self) -> Vec<QueueEntry> {` `crates/whisper-adapters/src/queue_store.rs:184`; `pub fn resolve_queued_with_catalog(` `crates/whisper-adapters/src/queue_store.rs:189`; `pub fn enqueue(&mut self, request: ImportRequest) -> Result<QueueEntry, String> {` `crates/whisper-adapters/src/queue_store.rs:219`; `pub fn set_status(` `crates/whisper-adapters/src/queue_store.rs:248`; `pub fn resume_interrupted(` `crates/whisper-adapters/src/queue_store.rs:316`; `pub fn validate_resume(` `crates/whisper-adapters/src/queue_store.rs:342`; `pub fn validate_resume_with_catalog(` `crates/whisper-adapters/src/queue_store.rs:390`; `pub fn resume_interrupted_with_catalog(` `crates/whisper-adapters/src/queue_store.rs:424`; `pub fn remove(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {` `crates/whisper-adapters/src/queue_store.rs:452`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/queue_store.rs:1-805`.

Utilisé par, via ces imports : tests : `crate::archive::tests` `crates/whisper-adapters/src/archive/tests/recovery.rs:542`<br>tests : `crate::archive::tests` `crates/whisper-adapters/src/archive/tests/recovery.rs:542`<br>tests : `crate::archive::tests` `crates/whisper-adapters/src/archive/tests/recovery.rs:542`<br>hors tests : `crate::recovery` `crates/whisper-adapters/src/recovery.rs:1`.

#### Inline `tests`

Tests/auxiliaires : `root` `crates/whisper-adapters/src/queue_store.rs:566-574`, `request` `crates/whisper-adapters/src/queue_store.rs:576-592`, `pending_compute_change_persists_only_the_choice_and_keeps_fifo` `crates/whisper-adapters/src/queue_store.rs:595-623`, `pending_compute_change_rejects_identity_or_status_changes` `crates/whisper-adapters/src/queue_store.rs:626-651`. `crates/whisper-adapters/src/queue_store.rs:561-767`

Mesure locale : 0 production / 207 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/queue_store.rs:561-767`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/queue_store.rs:561-767`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/queue_store.rs:561-767`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::recovery — `crates/whisper-adapters/src/recovery.rs`

Scanne archives/file et transforme les statuts pour un choix explicite de reprise. `crates/whisper-adapters/src/recovery.rs:1-45`

Mesure : 45 production / 0 tests / 45 physiques. `crates/whisper-adapters/src/recovery.rs:1-45`

Types : `RecoverySnapshot` (struct, hors tests) `crates/whisper-adapters/src/recovery.rs:9-12`.

Fonctions avec corps : `scan` `crates/whisper-adapters/src/recovery.rs:15-45`.

Déclarations `pub` (visibilité sous conditions) : `pub struct RecoverySnapshot {` `crates/whisper-adapters/src/recovery.rs:9`; `pub fn scan(destination: &Path) -> Result<RecoverySnapshot, String> {` `crates/whisper-adapters/src/recovery.rs:15`.

Dépend de, via `use crate::` : hors tests : `crate::archive::ArchiveEntry` → `crate::archive` `crates/whisper-adapters/src/recovery.rs:1`<br>hors tests : `crate::archive::ArchiveStore` → `crate::archive` `crates/whisper-adapters/src/recovery.rs:1`<br>hors tests : `crate::archive` → `crate::archive` `crates/whisper-adapters/src/recovery.rs:1`<br>hors tests : `crate::queue_store::QueueStore` → `crate::queue_store` `crates/whisper-adapters/src/recovery.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::source_binding — `crates/whisper-adapters/src/source_binding.rs`

Copie/identifie les sources et gère registre, réservations, reprise et retrait des snapshots. `crates/whisper-adapters/src/source_binding.rs:1-1716`

Mesure : 1519 production / 197 tests / 1716 physiques. `crates/whisper-adapters/src/source_binding.rs:1-1716`

Types : `SnapshotRecord` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:26-38`; `RegistryEntry` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:41-49`; `RetirementIntent` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:52-55`; `RecoveryArtifact` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:58-64`; `Reservation` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:66-69`; `RegistryStatus` (enum, hors tests) `crates/whisper-adapters/src/source_binding.rs:72-76`; `OperationGuardProbe` (enum, hors tests) `crates/whisper-adapters/src/source_binding.rs:78-82`; `SourceBinding` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:85-92`; `SnapshotRegistry` (struct, hors tests) `crates/whisper-adapters/src/source_binding.rs:1013-1015`.

Fonctions avec corps : `legacy_strategy` `crates/whisper-adapters/src/source_binding.rs:21-23`; `acquire` `crates/whisper-adapters/src/source_binding.rs:97-99`; `acquire_with_hook` `crates/whisper-adapters/src/source_binding.rs:101-338`; `reopen_for_identity` `crates/whisper-adapters/src/source_binding.rs:340-375`; `open_existing` `crates/whisper-adapters/src/source_binding.rs:377-415`; `snapshot_path` `crates/whisper-adapters/src/source_binding.rs:417-419`; `original_path` `crates/whisper-adapters/src/source_binding.rs:421-423`; `source_sha256` `crates/whisper-adapters/src/source_binding.rs:425-427`; `source_samples` `crates/whisper-adapters/src/source_binding.rs:429-431`; `strategy` `crates/whisper-adapters/src/source_binding.rs:433-435`; `apply_to_request` `crates/whisper-adapters/src/source_binding.rs:437-448`; `rebind` `crates/whisper-adapters/src/source_binding.rs:450-536`; `retire` `crates/whisper-adapters/src/source_binding.rs:539-569`; `retire_for_identity` `crates/whisper-adapters/src/source_binding.rs:574-598`; `inspect_recovery_candidates` `crates/whisper-adapters/src/source_binding.rs:603-623`; `recover_abandoned` `crates/whisper-adapters/src/source_binding.rs:627-672`; `recovery_candidate` `crates/whisper-adapters/src/source_binding.rs:675-812`; `inspect_bound_pair` `crates/whisper-adapters/src/source_binding.rs:814-841`; `recover_new_entry` `crates/whisper-adapters/src/source_binding.rs:843-878`; `resume_retirement` `crates/whisper-adapters/src/source_binding.rs:880-903`; `check_pending_intent` `crates/whisper-adapters/src/source_binding.rs:905-917`; `delete_verified_handle` `crates/whisper-adapters/src/source_binding.rs:919-928`; `delete_path_if_identity_matches` `crates/whisper-adapters/src/source_binding.rs:930-940`; `recovery_artifact` `crates/whisper-adapters/src/source_binding.rs:942-954`; `recovery_artifact_from_file` `crates/whisper-adapters/src/source_binding.rs:956-967`; `file_identity` `crates/whisper-adapters/src/source_binding.rs:969-975`; `expected_snapshot_path` `crates/whisper-adapters/src/source_binding.rs:977-982`; `expected_record_path` `crates/whisper-adapters/src/source_binding.rs:984-989`; `file_exists_strict` `crates/whisper-adapters/src/source_binding.rs:991-1000`; `hex_bytes` `crates/whisper-adapters/src/source_binding.rs:1002-1010`; `new` `crates/whisper-adapters/src/source_binding.rs:1018-1022`; `reserve` `crates/whisper-adapters/src/source_binding.rs:1024-1041`; `bind` `crates/whisper-adapters/src/source_binding.rs:1043-1071`; `rebind` `crates/whisper-adapters/src/source_binding.rs:1073-1105`; `release` `crates/whisper-adapters/src/source_binding.rs:1107-1129`; `read_entries` `crates/whisper-adapters/src/source_binding.rs:1131-1153`; `begin_retirement` `crates/whisper-adapters/src/source_binding.rs:1155-1182`; `remove_abandoned_empty` `crates/whisper-adapters/src/source_binding.rs:1184-1195`; `finish_retirement` `crates/whisper-adapters/src/source_binding.rs:1197-1215`; `update` `crates/whisper-adapters/src/source_binding.rs:1217-1262`; `validate_registry` `crates/whisper-adapters/src/source_binding.rs:1265-1284`; `registry_identity` `crates/whisper-adapters/src/source_binding.rs:1286-1295`; `registry_same_operation` `crates/whisper-adapters/src/source_binding.rs:1297-1301`; `capacity_error` `crates/whisper-adapters/src/source_binding.rs:1303-1305`; `ensure_destination` `crates/whisper-adapters/src/source_binding.rs:1307-1315`; `cleanup_failed_capture` `crates/whisper-adapters/src/source_binding.rs:1317-1335`; `snapshot_stem` `crates/whisper-adapters/src/source_binding.rs:1337-1339`; `lock_operation` `crates/whisper-adapters/src/source_binding.rs:1341-1356`; `recovery_operation_guard` `crates/whisper-adapters/src/source_binding.rs:1358-1381`; `operation_guard_state` `crates/whisper-adapters/src/source_binding.rs:1383-1389`; `read_record` `crates/whisper-adapters/src/source_binding.rs:1391-1397`; `validate_record` `crates/whisper-adapters/src/source_binding.rs:1399-1425`; `snapshot_schema` `crates/whisper-adapters/src/source_binding.rs:1427-1432`; `open_read_exclusive` `crates/whisper-adapters/src/source_binding.rs:1434-1443`; `open_verified_snapshot` `crates/whisper-adapters/src/source_binding.rs:1445-1464`; `open_disposition_snapshot` `crates/whisper-adapters/src/source_binding.rs:1466-1480`; `write_record_new` `crates/whisper-adapters/src/source_binding.rs:1482-1498`; `copy_and_hash` `crates/whisper-adapters/src/source_binding.rs:1500-1518`; `root` [tests] `crates/whisper-adapters/src/source_binding.rs:1525-1534`; `write_wav` [tests] `crates/whisper-adapters/src/source_binding.rs:1536-1552`; `request` [tests] `crates/whisper-adapters/src/source_binding.rs:1554-1570`; `snapshot_v2_preserves_strategy_and_rejects_request_mismatch` [tests] `crates/whisper-adapters/src/source_binding.rs:1573-1602`; `retirement_intent_replays_after_each_disposition_boundary` [tests] `crates/whisper-adapters/src/source_binding.rs:1605-1653`; `recovery_refuses_reserved_operation_while_acquire_holds_the_cross_process_lock` [tests] `crates/whisper-adapters/src/source_binding.rs:1656-1715`.

Déclarations `pub` (visibilité sous conditions) : `pub struct SourceBinding {` `crates/whisper-adapters/src/source_binding.rs:85`; `pub fn acquire(request: &ImportRequest, registry_root: &Path) -> Result<Self, String> {` `crates/whisper-adapters/src/source_binding.rs:97`; `pub fn reopen_for_identity(` `crates/whisper-adapters/src/source_binding.rs:340`; `pub fn snapshot_path(&self) -> &Path {` `crates/whisper-adapters/src/source_binding.rs:417`; `pub fn original_path(&self) -> &str {` `crates/whisper-adapters/src/source_binding.rs:421`; `pub fn source_sha256(&self) -> [u8; 32] {` `crates/whisper-adapters/src/source_binding.rs:425`; `pub fn source_samples(&self) -> u64 {` `crates/whisper-adapters/src/source_binding.rs:429`; `pub fn strategy(&self) -> whisper_core::parakeet_session::TranscriptionStrategy {` `crates/whisper-adapters/src/source_binding.rs:433`; `pub fn apply_to_request(&self, request: &ImportRequest) -> Result<ImportRequest, String> {` `crates/whisper-adapters/src/source_binding.rs:437`; `pub fn rebind(mut self, job_id: JobId, generation: Generation) -> Result<Self, String> {` `crates/whisper-adapters/src/source_binding.rs:450`; `pub fn retire(mut self) -> Result<(), String> {` `crates/whisper-adapters/src/source_binding.rs:539`; `pub fn retire_for_identity(` `crates/whisper-adapters/src/source_binding.rs:574`; `pub fn inspect_recovery_candidates(` `crates/whisper-adapters/src/source_binding.rs:603`; `pub fn recover_abandoned(` `crates/whisper-adapters/src/source_binding.rs:627`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/source_binding.rs:1-1716`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `recovery_tests`

Tests/auxiliaires : `root` `crates/whisper-adapters/src/source_binding.rs:1525-1534`, `write_wav` `crates/whisper-adapters/src/source_binding.rs:1536-1552`, `request` `crates/whisper-adapters/src/source_binding.rs:1554-1570`, `snapshot_v2_preserves_strategy_and_rejects_request_mismatch` `crates/whisper-adapters/src/source_binding.rs:1573-1602`. `crates/whisper-adapters/src/source_binding.rs:1520-1716`

Mesure locale : 0 production / 197 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/source_binding.rs:1520-1716`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/source_binding.rs:1520-1716`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/source_binding.rs:1520-1716`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::staging — `crates/whisper-adapters/src/staging.rs`

Écrit les blocs PCM et checkpoints durables via une file bornée. `crates/whisper-adapters/src/staging.rs:1-552`

Mesure : 459 production / 93 tests / 552 physiques. `crates/whisper-adapters/src/staging.rs:1-552`

Types : `PcmCheckpoint` (struct, hors tests) `crates/whisper-adapters/src/staging.rs:19-23`; `StoredCheckpoint` (struct, hors tests) `crates/whisper-adapters/src/staging.rs:26-29`; `PcmStaging` (struct, hors tests) `crates/whisper-adapters/src/staging.rs:31-40`; `WriterCommand` (enum, hors tests) `crates/whisper-adapters/src/staging.rs:42-46`; `PcmWriter` (struct, hors tests) `crates/whisper-adapters/src/staging.rs:50-56`.

Fonctions avec corps : `create` `crates/whisper-adapters/src/staging.rs:60-81`; `append_frame` `crates/whisper-adapters/src/staging.rs:83-105`; `append_tail` `crates/whisper-adapters/src/staging.rs:110-128`; `sync` `crates/whisper-adapters/src/staging.rs:130-182`; `with_campaign_identity` `crates/whisper-adapters/src/staging.rs:184-187`; `with_campaign_identity` `crates/whisper-adapters/src/staging.rs:189-191`; `drain` `crates/whisper-adapters/src/staging.rs:192-195`; `path` `crates/whisper-adapters/src/staging.rs:196-198`; `durable_samples` `crates/whisper-adapters/src/staging.rs:199-201`; `checksum` `crates/whisper-adapters/src/staging.rs:202-204`; `into_writer` `crates/whisper-adapters/src/staging.rs:206-219`; `inspect_checkpoint` `crates/whisper-adapters/src/staging.rs:221-270`; `persist_checkpoint` `crates/whisper-adapters/src/staging.rs:272-298`; `read_durable_samples` `crates/whisper-adapters/src/staging.rs:304-335`; `try_append_frame` `crates/whisper-adapters/src/staging.rs:338-352`; `append_frame` `crates/whisper-adapters/src/staging.rs:355-362`; `append_tail` `crates/whisper-adapters/src/staging.rs:364-376`; `drain` `crates/whisper-adapters/src/staging.rs:378-392`; `durable_samples` `crates/whisper-adapters/src/staging.rs:394-396`; `checksum` `crates/whisper-adapters/src/staging.rs:398-400`; `drop` `crates/whisper-adapters/src/staging.rs:404-409`; `writer_loop` `crates/whisper-adapters/src/staging.rs:412-439`; `validate_frame` `crates/whisper-adapters/src/staging.rs:441-450`; `checkpoint_path` `crates/whisper-adapters/src/staging.rs:452-458`; `periodic_pcm_sync_has_before_and_after_fault_points` [tests] `crates/whisper-adapters/src/staging.rs:465-551`.

Déclarations `pub` (visibilité sous conditions) : `pub const SYNC_EVERY_FRAMES: usize = 25;` `crates/whisper-adapters/src/staging.rs:14`; `pub const MAX_PENDING_FRAMES: usize = 50;` `crates/whisper-adapters/src/staging.rs:15`; `pub struct PcmCheckpoint {` `crates/whisper-adapters/src/staging.rs:19`; `pub struct PcmStaging {` `crates/whisper-adapters/src/staging.rs:31`; `pub struct PcmWriter {` `crates/whisper-adapters/src/staging.rs:50`; `pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {` `crates/whisper-adapters/src/staging.rs:60`; `pub fn append_frame(&mut self, samples: &[i16]) -> io::Result<u64> {` `crates/whisper-adapters/src/staging.rs:83`; `pub fn append_tail(&mut self, samples: &[i16]) -> io::Result<u64> {` `crates/whisper-adapters/src/staging.rs:110`; `pub fn sync(&mut self) -> io::Result<()> {` `crates/whisper-adapters/src/staging.rs:130`; `pub fn with_campaign_identity(mut self, job_id: u128, generation: u64) -> Self {` `crates/whisper-adapters/src/staging.rs:184`; `pub fn with_campaign_identity(self, _job_id: u128, _generation: u64) -> Self {` `crates/whisper-adapters/src/staging.rs:189`; `pub fn drain(&mut self) -> io::Result<u64> {` `crates/whisper-adapters/src/staging.rs:192`; `pub fn path(&self) -> &Path {` `crates/whisper-adapters/src/staging.rs:196`; `pub fn durable_samples(&self) -> u64 {` `crates/whisper-adapters/src/staging.rs:199`; `pub fn checksum(&self) -> [u8; 32] {` `crates/whisper-adapters/src/staging.rs:202`; `pub fn into_writer(self) -> PcmWriter {` `crates/whisper-adapters/src/staging.rs:206`; `pub fn inspect_checkpoint(path: impl AsRef<Path>) -> Result<PcmCheckpoint, String> {` `crates/whisper-adapters/src/staging.rs:221`; `pub fn read_durable_samples(` `crates/whisper-adapters/src/staging.rs:304`; `pub fn try_append_frame(&self, samples: Vec<i16>) -> io::Result<()> {` `crates/whisper-adapters/src/staging.rs:338`; `pub fn append_frame(&self, samples: Vec<i16>) -> io::Result<()> {` `crates/whisper-adapters/src/staging.rs:355`; `pub fn append_tail(&self, samples: Vec<i16>) -> io::Result<()> {` `crates/whisper-adapters/src/staging.rs:364`; `pub fn drain(&mut self) -> io::Result<u64> {` `crates/whisper-adapters/src/staging.rs:378`; `pub fn durable_samples(&self) -> u64 {` `crates/whisper-adapters/src/staging.rs:394`; `pub fn checksum(&self) -> [u8; 32] {` `crates/whisper-adapters/src/staging.rs:398`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/staging.rs:1-552`.

Utilisé par, via ces imports : hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`.

#### Inline `campaign_tests`

Tests/auxiliaires : `periodic_pcm_sync_has_before_and_after_fault_points` `crates/whisper-adapters/src/staging.rs:465-551`. `crates/whisper-adapters/src/staging.rs:460-552`

Mesure locale : 0 production / 93 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/staging.rs:460-552`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/staging.rs:460-552`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/staging.rs:460-552`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::supervisor — `crates/whisper-adapters/src/supervisor.rs`

Sélectionne les exécutables et suit progression et diagnostics tournants. `crates/whisper-adapters/src/supervisor.rs:1-676`

Mesure : 462 production / 214 tests / 676 physiques. `crates/whisper-adapters/src/supervisor.rs:1-676`

Types : `ProgressPhase` (enum, hors tests) `crates/whisper-adapters/src/supervisor.rs:17-26`; `WorkerPaths` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:29-32`; `WorkerSelection` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:35-38`; `DiagnosticPhase` (enum, hors tests) `crates/whisper-adapters/src/supervisor.rs:67-76`; `DiagnosticCode` (enum, hors tests) `crates/whisper-adapters/src/supervisor.rs:110-120`; `DiagnosticRecord` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:123-135`; `CampaignDiagnosticIdentity` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:139-142`; `DiagnosticSink` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:144-149`; `ProgressWatch` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:375-380`; `ProgressSupervisor` (struct, hors tests) `crates/whisper-adapters/src/supervisor.rs:424-426`; `TempDirectory` (struct, tests) `crates/whisper-adapters/src/supervisor.rs:475`.

Fonctions avec corps : `select` `crates/whisper-adapters/src/supervisor.rs:41-56`; `from` `crates/whisper-adapters/src/supervisor.rs:79-90`; `from` `crates/whisper-adapters/src/supervisor.rs:94-105`; `start` `crates/whisper-adapters/src/supervisor.rs:152-189`; `record` `crates/whisper-adapters/src/supervisor.rs:191-214`; `dropped_count` `crates/whisper-adapters/src/supervisor.rs:216-218`; `is_degraded` `crates/whisper-adapters/src/supervisor.rs:220-222`; `drop` `crates/whisper-adapters/src/supervisor.rs:226-231`; `new` `crates/whisper-adapters/src/supervisor.rs:235-259`; `with_detail` `crates/whisper-adapters/src/supervisor.rs:261-264`; `append_rotating` `crates/whisper-adapters/src/supervisor.rs:267-347`; `expire_old_diagnostics` `crates/whisper-adapters/src/supervisor.rs:349-361`; `remove_if_present` `crates/whisper-adapters/src/supervisor.rs:363-369`; `diagnostic_path` `crates/whisper-adapters/src/supervisor.rs:371-373`; `new` `crates/whisper-adapters/src/supervisor.rs:383-390`; `transition` `crates/whisper-adapters/src/supervisor.rs:392-399`; `observe` `crates/whisper-adapters/src/supervisor.rs:401-407`; `warning` `crates/whisper-adapters/src/supervisor.rs:409-420`; `starting` `crates/whisper-adapters/src/supervisor.rs:429-433`; `open` `crates/whisper-adapters/src/supervisor.rs:435-439`; `close` `crates/whisper-adapters/src/supervisor.rs:441-443`; `observe` `crates/whisper-adapters/src/supervisor.rs:445-450`; `take_warning` `crates/whisper-adapters/src/supervisor.rs:452-456`; `close_all` `crates/whisper-adapters/src/supervisor.rs:458-460`; `new` [tests] `crates/whisper-adapters/src/supervisor.rs:478-486`; `drop` [tests] `crates/whisper-adapters/src/supervisor.rs:490-492`; `record` [tests] `crates/whisper-adapters/src/supervisor.rs:495-504`; `startup_expires_only_known_stale_diagnostic_files` [tests] `crates/whisper-adapters/src/supervisor.rs:507-527`; `dropping_sink_drains_queued_records_and_closes_writer` [tests] `crates/whisper-adapters/src/supervisor.rs:530-540`; `bounded_queue_drops_without_blocking_and_marks_sink_degraded` [tests] `crates/whisper-adapters/src/supervisor.rs:543-563`; `capture_failure_diagnostic_keeps_the_technical_detail` [tests] `crates/whisper-adapters/src/supervisor.rs:566-579`; `rotation_stays_within_four_files_and_preserves_unknown_files` [tests] `crates/whisper-adapters/src/supervisor.rs:582-603`; `writer_io_failure_marks_sink_degraded` [tests] `crates/whisper-adapters/src/supervisor.rs:606-620`; `campaign_startup_purge_can_fail_before_deleting_old_diagnostics` [tests] `crates/whisper-adapters/src/supervisor.rs:624-675`.

Déclarations `pub` (visibilité sous conditions) : `pub enum ProgressPhase {` `crates/whisper-adapters/src/supervisor.rs:17`; `pub struct WorkerPaths {` `crates/whisper-adapters/src/supervisor.rs:29`; `pub struct WorkerSelection {` `crates/whisper-adapters/src/supervisor.rs:35`; `pub fn select(&self, choice: ComputeChoice) -> Result<WorkerSelection, ComputePolicyError> {` `crates/whisper-adapters/src/supervisor.rs:41`; `pub enum DiagnosticPhase {` `crates/whisper-adapters/src/supervisor.rs:67`; `pub enum DiagnosticCode {` `crates/whisper-adapters/src/supervisor.rs:110`; `pub struct DiagnosticRecord {` `crates/whisper-adapters/src/supervisor.rs:123`; `pub struct DiagnosticSink {` `crates/whisper-adapters/src/supervisor.rs:144`; `pub fn start(destination: &Path) -> Result<Self, std::io::Error> {` `crates/whisper-adapters/src/supervisor.rs:152`; `pub fn record(&self, record: DiagnosticRecord) {` `crates/whisper-adapters/src/supervisor.rs:191`; `pub fn dropped_count(&self) -> u64 {` `crates/whisper-adapters/src/supervisor.rs:216`; `pub fn is_degraded(&self) -> bool {` `crates/whisper-adapters/src/supervisor.rs:220`; `pub fn new(` `crates/whisper-adapters/src/supervisor.rs:235`; `pub fn with_detail(mut self, detail: String) -> Self {` `crates/whisper-adapters/src/supervisor.rs:261`; `pub struct ProgressWatch {` `crates/whisper-adapters/src/supervisor.rs:375`; `pub fn new(phase: ProgressPhase, now: Instant) -> Self {` `crates/whisper-adapters/src/supervisor.rs:383`; `pub fn transition(&mut self, phase: ProgressPhase, now: Instant) {` `crates/whisper-adapters/src/supervisor.rs:392`; `pub fn observe(&mut self, completed: u64, now: Instant) {` `crates/whisper-adapters/src/supervisor.rs:401`; `pub fn warning(&mut self, now: Instant) -> Option<(ProgressPhase, u64)> {` `crates/whisper-adapters/src/supervisor.rs:409`; `pub struct ProgressSupervisor {` `crates/whisper-adapters/src/supervisor.rs:424`; `pub fn starting(now: Instant) -> Self {` `crates/whisper-adapters/src/supervisor.rs:429`; `pub fn open(&mut self, phase: ProgressPhase, now: Instant) {` `crates/whisper-adapters/src/supervisor.rs:435`; `pub fn close(&mut self, phase: ProgressPhase) {` `crates/whisper-adapters/src/supervisor.rs:441`; `pub fn observe(&mut self, phase: ProgressPhase, completed: u64, now: Instant) {` `crates/whisper-adapters/src/supervisor.rs:445`; `pub fn take_warning(&mut self, now: Instant) -> Option<(ProgressPhase, u64)> {` `crates/whisper-adapters/src/supervisor.rs:452`; `pub fn close_all(&mut self) {` `crates/whisper-adapters/src/supervisor.rs:458`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/supervisor.rs:1-676`.

Utilisé par, via ces imports : hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`<br>hors tests : `crate::worker_ipc::backends` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:2`<br>hors tests : `crate::worker_ipc::service` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:9`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`.

#### Inline `tests`

Tests/auxiliaires : `new` `crates/whisper-adapters/src/supervisor.rs:478-486`, `drop` `crates/whisper-adapters/src/supervisor.rs:490-492`, `record` `crates/whisper-adapters/src/supervisor.rs:495-504`, `startup_expires_only_known_stale_diagnostic_files` `crates/whisper-adapters/src/supervisor.rs:507-527`. `crates/whisper-adapters/src/supervisor.rs:463-676`

Mesure locale : 0 production / 214 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/supervisor.rs:463-676`

Types locaux : `TempDirectory` `crates/whisper-adapters/src/supervisor.rs:475`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/supervisor.rs:463-676`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::vad — `crates/whisper-adapters/src/vad.rs`

Enveloppe WebRTC VAD pour les trames audio de 320 échantillons. `crates/whisper-adapters/src/vad.rs:1-32`

Mesure : 32 production / 0 tests / 32 physiques. `crates/whisper-adapters/src/vad.rs:1-32`

Types : `VoiceActivity` (struct, hors tests) `crates/whisper-adapters/src/vad.rs:5-7`.

Fonctions avec corps : `new` `crates/whisper-adapters/src/vad.rs:10-14`; `is_speech` `crates/whisper-adapters/src/vad.rs:16-25`; `default` `crates/whisper-adapters/src/vad.rs:29-31`.

Déclarations `pub` (visibilité sous conditions) : `pub const FRAME_SAMPLES: usize = 320;` `crates/whisper-adapters/src/vad.rs:3`; `pub struct VoiceActivity {` `crates/whisper-adapters/src/vad.rs:5`; `pub fn new() -> Self {` `crates/whisper-adapters/src/vad.rs:10`; `pub fn is_speech(&mut self, frame: &[i16]) -> Result<bool, String> {` `crates/whisper-adapters/src/vad.rs:16`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/vad.rs:1-32`.

Utilisé par, via ces imports : hors tests : `crate::worker_ipc` `crates/whisper-adapters/src/worker_ipc.rs:79`.

### crate::windows_file_identity — `crates/whisper-adapters/src/windows_file_identity.rs`

Ouvre les fichiers Windows, vérifie leur identité et les marque pour suppression. `crates/whisper-adapters/src/windows_file_identity.rs:1-314`

Mesure : 237 production / 77 tests / 314 physiques. `crates/whisper-adapters/src/windows_file_identity.rs:1-314`

Types : `FileIdentity` (struct, hors tests) `crates/whisper-adapters/src/windows_file_identity.rs:25-29`; `VerifiedFile` (struct, hors tests) `crates/whisper-adapters/src/windows_file_identity.rs:31-34`; `VerifiedSource` (struct, hors tests) `crates/whisper-adapters/src/windows_file_identity.rs:36-40`.

Fonctions avec corps : `open` `crates/whisper-adapters/src/windows_file_identity.rs:43-65`; `verify` `crates/whisper-adapters/src/windows_file_identity.rs:67-73`; `sha256` `crates/whisper-adapters/src/windows_file_identity.rs:75-92`; `read_all` `crates/whisper-adapters/src/windows_file_identity.rs:94-104`; `mark_delete` `crates/whisper-adapters/src/windows_file_identity.rs:106-146`; `identity_for_handle` `crates/whisper-adapters/src/windows_file_identity.rs:149-187`; `inspect` `crates/whisper-adapters/src/windows_file_identity.rs:189-192`; `delete_if_identity_matches` `crates/whisper-adapters/src/windows_file_identity.rs:194-201`; `open_and_hash_source` `crates/whisper-adapters/src/windows_file_identity.rs:203-226`; `sha256_hex` `crates/whisper-adapters/src/windows_file_identity.rs:228-236`; `sandbox` [tests] `crates/whisper-adapters/src/windows_file_identity.rs:243-254`; `deletion_is_applied_to_the_open_identity` [tests] `crates/whisper-adapters/src/windows_file_identity.rs:257-270`; `mismatching_identity_is_not_deleted` [tests] `crates/whisper-adapters/src/windows_file_identity.rs:273-290`; `verified_handle_blocks_path_substitution_until_disposition` [tests] `crates/whisper-adapters/src/windows_file_identity.rs:293-313`.

Déclarations `pub` (visibilité sous conditions) : `pub fn open(path: &Path, directory: bool) -> Result<Self, String> {` `crates/whisper-adapters/src/windows_file_identity.rs:43`; `pub fn verify(&self, expected: &FileIdentity) -> Result<(), String> {` `crates/whisper-adapters/src/windows_file_identity.rs:67`; `pub fn sha256(&mut self) -> Result<String, String> {` `crates/whisper-adapters/src/windows_file_identity.rs:75`; `pub fn read_all(&mut self) -> Result<Vec<u8>, String> {` `crates/whisper-adapters/src/windows_file_identity.rs:94`; `pub fn mark_delete(` `crates/whisper-adapters/src/windows_file_identity.rs:106`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/windows_file_identity.rs:1-314`.

Utilisé par, via ces imports : hors tests : `crate::archive::cleanup::execution` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::archive::cleanup::execution` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::archive::cleanup::execution` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::archive::cleanup::execution` `crates/whisper-adapters/src/archive/cleanup/execution.rs:31`<br>hors tests : `crate::archive::cleanup::proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:23`<br>hors tests : `crate::archive::cleanup::proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:203`<br>hors tests : `crate::archive::cleanup::proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:295`<br>hors tests : `crate::archive::cleanup::proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:328`<br>hors tests : `crate::archive::cleanup::proofs` `crates/whisper-adapters/src/archive/cleanup/proofs.rs:382`.

#### Inline `tests`

Tests/auxiliaires : `sandbox` `crates/whisper-adapters/src/windows_file_identity.rs:243-254`, `deletion_is_applied_to_the_open_identity` `crates/whisper-adapters/src/windows_file_identity.rs:257-270`, `mismatching_identity_is_not_deleted` `crates/whisper-adapters/src/windows_file_identity.rs:273-290`, `verified_handle_blocks_path_substitution_until_disposition` `crates/whisper-adapters/src/windows_file_identity.rs:293-313`. `crates/whisper-adapters/src/windows_file_identity.rs:238-314`

Mesure locale : 0 production / 77 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/windows_file_identity.rs:238-314`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/windows_file_identity.rs:238-314`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/windows_file_identity.rs:238-314`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::worker_ipc::backends — `crates/whisper-adapters/src/worker_ipc/backends/mod.rs`

Lie les demandes au catalogue, qualifie le GPU et suit les workers réutilisables. `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:1-468`

Mesure : 468 production / 0 tests / 468 physiques. `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:1-468`

Types : `RuntimeModelCatalog` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:96-101`; `IdleWorker` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:431-437`.

Fonctions avec corps : `probe_cuda_worker` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:21-52`; `qualify_gpu_worker` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:55-94`; `clone_for_worker` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:104-113`; `snapshot` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:115-117`; `bind_import` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:119-223`; `bind_live` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:225-231`; `bind_native_unified_live` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:233-258`; `bind_parakeet_live` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:260-334`; `is_parakeet_live_request` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:336-340`; `request_is_parakeet` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:343-350`; `validate_current_queued_gpu_candidate` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:352-366`; `validate_queued_gpu_preflight` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:368-385`; `archive_qualification_for_request` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:387-427`; `matches` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:440-451`; `matches_identity` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:453-467`.

Déclarations `pub` (visibilité sous conditions) : `pub fn probe_cuda_worker(executable: &Path) -> Result<(), String> {` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:21`; `pub fn qualify_gpu_worker(` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:55`.

Dépend de, via `use crate::` : hors tests : `crate::WorkerTransport` → [Non vérifié] symbole/réexport non résolu `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:2`<br>hors tests : `crate::supervisor::WorkerPaths` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::backends::native — `crates/whisper-adapters/src/worker_ipc/backends/native.rs`

Résout les routes natives unifiées CPU/GPU et leurs preuves. `crates/whisper-adapters/src/worker_ipc/backends/native.rs:1-215`

Mesure : 215 production / 0 tests / 215 physiques. `crates/whisper-adapters/src/worker_ipc/backends/native.rs:1-215`

Types : `NativeUnifiedRoute` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/backends/native.rs:6-13`.

Fonctions avec corps : `same_worker` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:16-24`; `backend_label` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:26-35`; `can_retry_native_auto_on_cpu` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:38-48`; `native_unified_route` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:50-135`; `native_cpu_route` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:137-165`; `gpu_qualification_candidate_allowed` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:167-194`; `gpu_evidence_matches` `crates/whisper-adapters/src/worker_ipc/backends/native.rs:196-215`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/backends/native.rs:1-215`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/backends/native.rs:1-215`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::ipc::framing — `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs`

Lit, écrit et sérialise les trames JSON bornées. `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:1-106`

Mesure : 106 production / 0 tests / 106 physiques. `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:1-106`

Types : `BoundedFrameWriter` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:74-76`.

Fonctions avec corps : `read_frame` `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:8-53`; `write_frame` `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:54-72`; `write` `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:79-92`; `flush` `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:94-96`; `serialize_frame` `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:99-106`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:1-106`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:1-106`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::ipc — `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs`

Lance le worker et gère les files bornées de commandes/événements. `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:1-398`

Mesure : 382 production / 16 tests / 398 physiques. `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:1-398`

Types : `ChildWorkerTransport` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:29-39`; `ExpectedGpuSession` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:42-47`; `WorkerReaderMessage` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:50-53`; `WorkerCommandPoll` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:303-307`; `EnqueueError` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:330-333`.

Fonctions avec corps : `spawn` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:56-143`; `inject_event` [tests] `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:146-148`; `try_wait` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:150-154`; `abort` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:156-159`; `drop` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:163-165`; `request` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:169-211`; `receive` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:213-290`; `try_request` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:294-296`; `try_request_ordered` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:298-300`; `take_next_worker_command` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:309-327`; `try_enqueue_worker_command` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:335-346`; `try_enqueue_bounded` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:348-356`; `read_events` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:358-398`.

Déclarations `pub` (visibilité sous conditions) : `pub struct ChildWorkerTransport {` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:29`; `pub fn spawn(executable: &Path) -> Result<Self, PortError> {` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:56`; `pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, PortError> {` `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:150`.

Dépend de, via `use crate::` : hors tests : `crate::WorkerTransport` → [Non vérifié] symbole/réexport non résolu `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:12`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::parakeet_unified::executor — `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs`

Gère le thread d’inférence unifié, ses commandes et le cycle du processus. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:1-462`

Mesure : 305 production / 157 tests / 462 physiques. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:1-462`

Types : `UnifiedInferenceCommand` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:17-22`; `UnifiedInferenceResult` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:24-28`; `UnifiedExecutorCommand` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:30-33`; `UnifiedInferenceExecutor` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:35-60`.

Fonctions avec corps : `is_available` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:63-65`; `start` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:67-78`; `start_observed` [tests] `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:81-86`; `start_observed_with_ready_gate` [tests] `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:89-95`; `start_inner` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:97-132`; `start_instance` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:134-319`; `is_ready` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:321-333`; `dispatch` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:335-347`; `try_result` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:349-363`; `stop_and_reap` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:365-400`; `reap_disconnected` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:402-440`; `restart` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:442-447`; `replace_reaped_worker` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:449-461`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:1-462`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:1-462`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::parakeet_unified — `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs`

Définit sources/sessions unifiées et restaure le préfixe documentaire. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:1-208`

Mesure : 200 production / 8 tests / 208 physiques. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:1-208`

Types : `UnifiedDocumentDrain` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:14-23`; `UnifiedSessionSource` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:105-108`; `UnifiedSessionState` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:185-208`.

Fonctions avec corps : `restore_unified_document_prefix` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:25-103`; `read_window` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:111-116`; `is_capture` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:120-122`; `pump_capture` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:124-131`; `stop_capture` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:133-140`; `set_gain` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:142-152`; `take_probe_telemetry` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:154-161`; `take_gain_events` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:162-169`; `enqueue_test_block` [tests] `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:172-176`; `pump` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:178-182`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:1-208`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:1-208`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::parakeet_unified::native — `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs`

Vérifie le PCM et exécute les appels Parakeet corrélés. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:1-465`

Mesure : 342 production / 123 tests / 465 physiques. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:1-465`

Types : `NativeParakeetWindowTranscriber` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:36-45`.

Fonctions avec corps : `validate_native_window_pcm` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:14-34`; `drop` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:48-52`; `new` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:56-67`; `with_harness_observer` [tests] `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:70-76`; `model_sha256` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:78-82`; `verify_ready` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:84-141`; `instance_id` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:145-149`; `initialize_instance` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:151-249`; `stop_instance` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:251-273`; `transcribe` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:275-281`; `transcribe_scheduled` `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:283-464`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:1-465`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:1-465`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::publication — `crates/whisper-adapters/src/worker_ipc/publication/mod.rs`

Arbitre atomiquement activation, annulation et engagement de publication. `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:1-73`

Mesure : 71 production / 2 tests / 73 physiques. `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:1-73`

Types : `PublicationGate` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:11`.

Fonctions avec corps : `activate` `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:14-16`; `is_cancelled` `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:18-20`; `request_stop` `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:22-35`; `rollback_stop` `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:37-44`; `commit` `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:46-69`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:1-73`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/publication/mod.rs:1-73`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::publication::tests — `crates/whisper-adapters/src/worker_ipc/publication/tests.rs`

Tests ou supports : définit `failed_commit_remains_committing_even_after_stop_rollback` `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:4-17`, `rollback_only_restores_a_cancelled_gate` `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:20-29`.

Mesure : 0 production / 29 tests / 29 physiques. `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:1-29`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:1-29`.

Fonctions avec corps : `failed_commit_remains_committing_even_after_stop_rollback` [tests] `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:4-17`; `rollback_only_restores_a_cancelled_gate` [tests] `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:20-29`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:1-29`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/publication/tests.rs:1-29`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::duplicates — `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs`

Traite doublons et reconfirmations de publications interrompues. `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:1-423`

Mesure : 423 production / 0 tests / 423 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:1-423`

Types : `DuplicateEffect` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:3-6`.

Fonctions avec corps : `resolve_reprocessing_targets` `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:8-84`; `verified_present_publications` `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:86-93`; `confirm_interrupted_publication` `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:96-214`; `confirm_duplicate_import` `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:216-422`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:1-423`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:1-423`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::effects — `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs`

Distribue les effets et convertit les erreurs en événements d’échec. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237`

Mesure : 237 production / 0 tests / 237 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237`.

Fonctions avec corps : `effect` `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:4-236`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:1-237`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::import::worker_start — `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs`

Démarre les workers Turbo/Parakeet et applique les replis CPU. `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:1-665`

Mesure : 665 production / 0 tests / 665 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:1-665`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:1-665`.

Fonctions avec corps : `start_worker_with_candidate` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:4-10`; `start_worker_with_cpu_override` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:12-18`; `start_worker_with_candidate_override` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:20-188`; `start_parakeet_worker` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:190-450`; `replace_parakeet_pending_with_cpu_key` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:452-465`; `fallback_parakeet_to_cpu` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:467-492`; `handle_worker_failure` `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:494-664`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:1-665`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/import/worker_start.rs:1-665`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::import — `crates/whisper-adapters/src/worker_ipc/runtime/import.rs`

Pilote préparation, persistance, publication et admission des imports unifiés. `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1-1438`

Mesure : 1205 production / 233 tests / 1438 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1-1438`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1-1438`.

Fonctions avec corps : `prepare` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:6-127`; `start_worker` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:129-134`; `start_worker_with_selection` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:136-173`; `persist_segment` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:175-241`; `publish` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:243-277`; `publish_inner` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:279-417`; `archive_root` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:418-420`; `stop` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:421-582`; `set_gpu_candidate` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:583-592`; `read_unified_document_page_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:593-625`; `read_unified_audio_parts_page_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:626-658`; `start_worker_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:659-704`; `start_automatic_final_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:705-772`; `scan_history_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:773-809`; `resolve_import_publication_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:810-829`; `resolve_live_publication_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:830-847`; `persist_segment_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:848-871`; `stop_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:872-991`; `prepare_effect` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:992-1011`; `register_prepared_native_route` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1015-1046`; `native_route_for_unified_import` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1048-1081`; `is_controlled_bounded_test` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1083-1100`; `observe_source_binding_map` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1103-1113`; `start_controlled_unified_import` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1115-1251`; `run_controlled_unified_import` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1254-1427`; `bind_import_request` `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1429-1437`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1-1438`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:1-1438`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::live — `crates/whisper-adapters/src/worker_ipc/runtime/live.rs`

Pilote plusieurs chemins live, gain, tests micro et arrêts. `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1-1380`

Mesure : 1353 production / 27 tests / 1380 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1-1380`

Types : `FinishedLiveWindow` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:32-36`; `LiveRecoveryState` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:38-43`; `LiveCaptureState` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:44-77`; `PendingLiveState` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:84-89`; `ImportRequestState` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:90-102`; `LiveEffect` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:420-423`; `MicrophoneTestCapture` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:500-511`.

Fonctions avec corps : `live_capture_poll_allowed` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:4-9`; `campaign_reserve_attempt` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:11-29`; `stop_capture` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:79-82`; `start_unified_capture` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:105-200`; `validate_unified_capture_admission` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:202-265`; `start_native_unified_capture_with` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:267-284`; `stop_controlled_unified_capture` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:286-357`; `bind_live_request` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:359-366`; `confirm_live_window` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:368-417`; `spawn_live_attempt` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:425-498`; `start` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:514-524`; `from_capture` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:526-557`; `set_gain` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:559-569`; `pump` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:571-631`; `stop` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:633-640`; `start_live` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:644-752`; `activate_live_after_ready` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:754-821`; `start_cpu_live_attempt` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:822-947`; `start_cpu_pending_live_attempt` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:949-1021`; `finish_live_on_cpu` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1023-1045`; `start_microphone_test` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1047-1081`; `start_direct_live` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1082-1168`; `start_unified_live` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1169-1244`; `start_legacy_live` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1245-1262`; `set_microphone_gain` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1263-1325`; `finish_live_on_cpu_effect` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1326-1339`; `stop_unified_live_effect` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1340-1361`; `stop_microphone_test` `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1362-1379`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1-1380`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1-1380`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime — `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs`

Porte ImportRuntime et compose dépendances et registres. `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:1-269`

Mesure : 171 production / 98 tests / 269 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:1-269`

Types : `ImportRuntime` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:46-139`.

Fonctions avec corps : `new` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:143-145`; `new_with_catalog` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:148-154`; `new_with_workers_and_parakeet` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:157-174`; `new_with_workers_and_parakeet_registry` `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:176-268`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:1-269`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:1-269`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::poll::capture — `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs`

Pompe capture live, fenêtres et télémétrie. `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:1-254`

Mesure : 254 production / 0 tests / 254 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:1-254`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:1-254`.

Fonctions avec corps : `poll_live_capture` `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:4-106`; `pump_live_windows` `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:108-234`; `take_legacy_live_telemetry` `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:236-253`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:1-254`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs:1-254`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::poll::parakeet — `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs`

Draine Parakeet historique, VAD et progression des fenêtres import. `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:1-996`

Mesure : 996 production / 0 tests / 996 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:1-996`

Types : `ParakeetVadResult` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:83-92`; `ParakeetVadRun` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:94-112`; `ParakeetStoppedExit` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:123-127`.

Fonctions avec corps : `vad_word_overlap_within_one_tdt_frame` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:4-9`; `append_vad_parakeet_word_group` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:11-33`; `validate_single_pass_parakeet_words` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:35-59`; `append_single_pass_parakeet_word_group` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:61-82`; `drop` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:115-119`; `parakeet_stopped_exit` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:129-135`; `parakeet_vad_segments` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:137-189`; `advance_parakeet_vad_decode` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:192-297`; `begin_next_parakeet_vad_window` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:299-396`; `handle_parakeet_vad_result` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:398-485`; `confirm_and_advance_parakeet_vad_window` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:487-504`; `poll_parakeet_worker` `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:506-995`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:1-996`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:1-996`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::poll — `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs`

Draine les workers et route leurs événements avec contrôles d’identité. `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:1-1232`

Mesure : 1226 production / 6 tests / 1232 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:1-1232`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:1-1232`.

Fonctions avec corps : `take_ready_live_stop` `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:13-20`; `bind_unidentified_startup_failure` `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:22-46`; `worker_event_identity` `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:48-113`; `worker_event_is_stale` `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:115-120`; `map_attempt_sequence` `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:122-131`; `poll` `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:134-1231`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:1-1232`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:1-1232`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::poll_unified — `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs`

Draine sessions/documents unifiés et traite les états terminaux. `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:1-1374`

Mesure : 1312 production / 62 tests / 1374 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:1-1374`

Types : `NativeAutoCpuRetry` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:3-6`; `FailedManualGpuPeer` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:8-11`; `NativeRetryState` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:21-30`.

Fonctions avec corps : `manual_gpu_failure_event` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:13-19`; `retry_native_gpu_startup_on_cpu` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:32-221`; `release_unified_terminal_state` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:224-254`; `unified_admission_full` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:256-270`; `unified_admission_full_for` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:272-317`; `poll_unified_document` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:319-380`; `poll_unified_sessions` `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:382-1373`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:1-1374`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:1-1374`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::queue — `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs`

Ajoute, retire, prépare et scanne les éléments de la file. `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:1-423`

Mesure : 423 production / 0 tests / 423 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:1-423`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:1-423`.

Fonctions avec corps : `enqueue` `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:4-166`; `remove_queued` `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:167-191`; `process_queued` `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:192-409`; `scan_queue` `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:410-422`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:1-423`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:1-423`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::recovery — `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs`

Valide et prépare la reprise import et le retrait de remplacement. `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:1-369`

Mesure : 351 production / 18 tests / 369 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:1-369`

Types : `RecoveryEffect` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:3-6`.

Fonctions avec corps : `resume_interrupted` `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:9-349`; `resume_replacement_cleanup` `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:350-368`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:1-369`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:1-369`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::source_binding — `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs`

Gère bindings source, transfert à la reprise et récupération. `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:1-529`

Mesure : 523 production / 6 tests / 529 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:1-529`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:1-529`.

Fonctions avec corps : `ensure_source_binding` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:4-39`; `rebind_source_binding` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:41-63`; `source_binding_matches_resume` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:65-84`; `transfer_resume_source_binding` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:86-95`; `transfer_resume_source_binding_inner` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:97-243`; `restore_resume_source_binding` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:245-254`; `restore_resume_source_binding_inner` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:256-308`; `release_source_binding` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:310-320`; `release_source_binding_inner` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:322-451`; `active_snapshot_bindings` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:453-461`; `inspect_source_snapshot_recovery` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:463-474`; `inspect_snapshot_recovery_effect` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:476-492`; `confirm_snapshot_recovery_effect` `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:493-528`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:1-529`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:1-529`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::runtime::support — `crates/whisper-adapters/src/worker_ipc/runtime/support.rs`

Fournit identités, barrières de recette, historique et adaptation ServiceRuntime. `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:1-456`

Mesure : 365 production / 91 tests / 456 physiques. `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:1-456`

Types : `PreflightResult` (enum, hors tests) `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:3-10`.

Fonctions avec corps : `collision_free_job_id` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:12-49`; `acceptance_barrier` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:54-93`; `acceptance_barrier` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:96-101`; `take_acceptance_worker_fault` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:104-154`; `canonical_extended_acceptance_root` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:157-176`; `unix_time_ms` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:178-184`; `scan_archive_history` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:186-241`; `queued_request_identity_matches` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:243-258`; `pending_unified_event_identity` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:260-287`; `recovery_stop_counters` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:289-295`; `start_diagnostics` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:298-309`; `with_controlled_transcriber` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:312-318`; `with_disconnected_result_for` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:321-327`; `with_panicking_transcriber_for` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:330-336`; `with_finish_document_failure_for` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:339-345`; `with_resume_binding_transfer_gate` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:348-354`; `hold_next_reserved_call_once` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:357-359`; `hold_next_child_ready` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:362-368`; `with_child_observer` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:371-377`; `with_unrecycled_child_mutant` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:380-387`; `with_controlled_capture` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:390-402`; `with_native_harness_observer` [tests] `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:405-411`; `apply_effect` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:415-428`; `poll_event` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:430-432`; `pump_realtime` `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:434-455`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:1-456`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:1-456`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::service::automatic_final — `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs`

Attache l’import automatique et installe exports et états de manifeste. `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:1-320`

Mesure : 320 production / 0 tests / 320 physiques. `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:1-320`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:1-320`.

Fonctions avec corps : `next_automatic_final_job_id` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:13-31`; `inspect_automatic_final_queue` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:33-39`; `inspect_automatic_final_queue_with` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:41-46`; `attach_automatic_final_import` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:48-101`; `publish_automatic_final_exports` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:103-159`; `set_automatic_final_manifest_status` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:161-224`; `create_automatic_stage` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:226-248`; `remove_automatic_stage` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:250-259`; `remove_automatic_paths` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:261-276`; `with_cleanup_error` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:278-283`; `publish_automatic_staged_pair` `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:285-320`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:1-320`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:1-320`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::service — `crates/whisper-adapters/src/worker_ipc/service/mod.rs`

Implémente le port asynchrone et les boucles de service à files bornées. `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1-812`

Mesure : 767 production / 45 tests / 812 physiques. `crates/whisper-adapters/src/worker_ipc/service/mod.rs:1-812`

Types : `AsyncImportIo` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc/service/mod.rs:25-41`; `ServiceRuntime` (trait, hors tests) `crates/whisper-adapters/src/worker_ipc/service/mod.rs:646-652`.

Fonctions avec corps : `start` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:44-53`; `start_with_workers` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:55-57`; `start_with_model_catalog` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:60-75`; `start_with_model_catalog_and_parakeet` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:77-90`; `start_with_model_catalog_and_parakeet_routes` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:92-109`; `start_with_model_catalog_and_parakeet_routes_and_snapshot_registry_native_candidate` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:113-133`; `start_with_model_catalog_and_parakeet_routes_and_snapshot_registry` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:137-157`; `start_with_model_catalog_and_parakeet_routes_inner` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:159-175`; `start_inner` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:177-256`; `for_test` [tests] `crates/whisper-adapters/src/worker_ipc/service/mod.rs:259-302`; `start_next_automatic` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:304-330`; `import_effect_identity` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:333-391`; `terminal_import_event_identity` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:393-408`; `submit` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:411-596`; `poll` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:597-643`; `pump_realtime` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:649-651`; `effect_event_reservation` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:654-661`; `service_loop` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:663-812`.

Déclarations `pub` (visibilité sous conditions) : `pub struct AsyncImportIo {` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:25`; `pub fn start(worker_path: PathBuf) -> Self {` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:44`; `pub fn start_with_workers(workers: WorkerPaths) -> Self {` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:55`; `pub fn start_with_model_catalog(` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:60`; `pub fn start_with_model_catalog_and_parakeet(` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:77`; `pub fn start_with_model_catalog_and_parakeet_routes(` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:92`; `pub fn start_with_model_catalog_and_parakeet_routes_and_snapshot_registry_native_candidate(` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:113`; `pub fn start_with_model_catalog_and_parakeet_routes_and_snapshot_registry(` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:137`.

Dépend de, via `use crate::` : hors tests : `crate::supervisor::WorkerPaths` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc/service/mod.rs:9`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::support — `crates/whisper-adapters/src/worker_ipc/support.rs`

Fournit trace mémoire conditionnelle, conversions SHA-256 et écriture WAV. `crates/whisper-adapters/src/worker_ipc/support.rs:1-94`

Mesure : 94 production / 0 tests / 94 physiques. `crates/whisper-adapters/src/worker_ipc/support.rs:1-94`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/support.rs:1-94`.

Fonctions avec corps : `memory_trace` `crates/whisper-adapters/src/worker_ipc/support.rs:16-40`; `decode_sha256` `crates/whisper-adapters/src/worker_ipc/support.rs:42-52`; `format_sha256` `crates/whisper-adapters/src/worker_ipc/support.rs:54-56`; `write_pcm16_wav` `crates/whisper-adapters/src/worker_ipc/support.rs:61-94`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/support.rs:1-94`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/support.rs:1-94`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs`

Tests ou supports : définit `duplicate_confirmation_short_refusals_preserve_runtime_identity` `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:2-50`.

Mesure : 0 production / 50 tests / 50 physiques. `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:1-50`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:1-50`.

Fonctions avec corps : `duplicate_confirmation_short_refusals_preserve_runtime_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:2-50`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:1-50`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs:1-50`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs`

Tests ou supports : définit `automatic_final_binding_failure_updates_companion_and_clears_identity` `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:2-59`, `pagination_effects_preserve_request_and_generation_identity_on_error` `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:62-105`.

Mesure : 0 production / 105 tests / 105 physiques. `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:1-105`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:1-105`.

Fonctions avec corps : `automatic_final_binding_failure_updates_companion_and_clears_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:2-59`; `pagination_effects_preserve_request_and_generation_identity_on_error` [tests] `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:62-105`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:1-105`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/effect_characterization.rs:1-105`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs`

Tests ou supports : définit `controlled_worker_and_invalid_times_leave_no_complete_archive` `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:2-76`, `export_collision_keeps_existing_file_and_does_not_publish_unified_archive` `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:79-160`.

Mesure : 0 production / 2068 tests / 2068 physiques. `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1-2068`

Types : `SimulatedPort` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:435`; `IncrementalWavCapture` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:797-802`; `SampleDrivenWavCapture` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:864-871`; `RuntimeAutomaticPublisher` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:937-945`; `SlowAutomaticFinal` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1065-1067`; `BlockedAutomaticQueueInspection` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1087-1091`.

Fonctions avec corps : `controlled_worker_and_invalid_times_leave_no_complete_archive` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:2-76`; `export_collision_keeps_existing_file_and_does_not_publish_unified_archive` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:79-160`; `microphone_legacy_capture_budget_forces_worker_poll_within_four_ui_events` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:163-190`; `microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:193-430`; `microphone_gain_command_reaches_acquisition_and_finalized_wav_while_worker_is_delayed` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:433-608`; `submit` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:438-441`; `poll` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:443-445`; `stopping_before_the_first_gain_revision_block_emits_no_applied_acknowledgement` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:611-630`; `microphone_test_capture_owns_only_capture_conversion_and_probe_until_stop` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:633-708`; `automatic_final_test_root` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:710-718`; `automatic_final_request` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:720-733`; `wait_for_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:735-756`; `direct_live_request` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:758-774`; `write_test_wav_header` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:776-795`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:805-833`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:835-837`; `pump_realtime` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:839-861`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:874-905`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:907-909`; `pump_realtime` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:911-934`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:948-1045`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1047-1049`; `write_fixture_wav` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1052-1063`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1070-1080`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1082-1084`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1094-1117`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1119-1121`; `live_slow_automatic_final_failure_does_not_block_another_capture_wav` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1125-1244`; `live_blocked_automatic_queue_inspection_does_not_block_capture_b_stop_or_wav` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1247-1467`; `live_automatic_final_queue_corruption_is_reported_explicitly` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1470-1480`; `live_automatic_final_gate_does_not_block_next_session_stop_or_late_publish` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1483-1859`; `live_automatic_final_failure_is_routed_without_changing_active_capture` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1862-1931`; `parakeet_live_automatic_final_exports_are_distinct_create_new_outputs` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1934-2004`; `automatic_final_publication_failure_leaves_no_partial_public_name` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:2007-2026`; `automatic_final_source_read_failure_publishes_neither_output` [tests] `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:2029-2068`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1-2068`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs:1-2068`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs`

Tests ou supports : définit `native_window_pcm_requires_a_bounded_matching_riff_range` `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:2-37`, `native_window_wav_is_short_pcm16_mono_16khz_riff` `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:40-68`.

Mesure : 0 production / 3980 tests / 3980 physiques. `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1-3980`

Types : `NativeLongAnnotation` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:810-818`; `NativeLongAnnotationSegment` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:822-827`; `ManifestArtifact` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:899-903`; `CompleteManifest` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:906-914`.

Fonctions avec corps : `native_window_pcm_requires_a_bounded_matching_riff_range` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:2-37`; `native_window_wav_is_short_pcm16_mono_16khz_riff` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:40-68`; `failed_ready_is_reaped_before_the_executor_recycles` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:71-114`; `native_auto_gpu_ready_failure_retries_once_on_cpu_before_first_call` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:117-561`; `prepared_native_routes_without_start_are_bounded_and_stop_releases_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:564-664`; `native_executor_backend_changes_only_after_the_prior_instance_is_reaped` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:667-765`; `native_personal_gpu_candidate_for_harness` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:768-772`; `native_personal_gpu_candidate_for_generation_harness` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:775-806`; `native_queued_import_source` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:830-891`; `assert_native_generation_text_artifacts` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:894-965`; `run_native_queued_unified_import` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:968-1341`; `native_spoken_prefix` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1344-1431`; `source_binding_map_events` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1433-1447`; `native_event_drain` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1450-1464`; `run_native_personal_ab_c` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1467-3109`; `phase_elapsed_or_unobserved` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3067-3072`; `native_personal_cpu_ab_c_recycles_after_reap_with_spoken_burst_capture` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3113-3115`; `native_personal_gpu_ab_c_recycles_after_reap_with_attested_candidate` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3120-3132`; `native_personal_cpu_finalizes_a_synthetic_capture_without_opening_hardware` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3137-3294`; `native_queued_cpu_short_import_uses_the_durable_application_path` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3299-3301`; `native_queued_gpu_short_import_uses_current_attestation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3306-3308`; `native_queued_gpu_resume_uses_current_attestation_and_rebinds_snapshot` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3313-3945`; `native_queued_auto_short_import_prefers_current_gpu_candidate` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3950-3952`; `native_queued_auto_short_import_falls_back_to_cpu_without_candidate` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3957-3959`; `native_queued_cpu_long_import_uses_the_durable_application_path` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3964-3966`; `native_queued_gpu_long_import_uses_current_attestation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3971-3973`; `native_queued_auto_long_import_prefers_current_gpu_candidate` [tests] `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3978-3980`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1-3980`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1-3980`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs`

Tests ou supports : définit `queue_effect_scan_and_remove_preserve_remaining_fifo_entry` `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:2-74`.

Mesure : 0 production / 74 tests / 74 physiques. `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:1-74`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:1-74`.

Fonctions avec corps : `queue_effect_scan_and_remove_preserve_remaining_fifo_entry` [tests] `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:2-74`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:1-74`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/queue_characterization.rs:1-74`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs`

Tests ou supports : définit `emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets` `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:31-63`, `reprocessing_latest_requires_known_unique_order_and_all_includes_inactive_present` `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:66-104`.

Mesure : 0 production / 2296 tests / 2296 physiques. `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1-2296`

Types : `Runtime` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1546`; `Runtime` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1642-1645`; `Runtime` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1727`; `Runtime` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1788-1793`; `WireRuntime` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2044-2048`.

Fonctions avec corps : `reprocessing_candidate` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1-17`; `reprocessing_test_root` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:19-28`; `emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:31-63`; `reprocessing_latest_requires_known_unique_order_and_all_includes_inactive_present` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:66-104`; `browse_accepts_only_exact_verified_treatment_in_current_destination` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:107-157`; `test_artifact_root` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:159-164`; `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:166-180`; `timestamp_guard_accepts_exactly_one_tdt_frame_overlap` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:183-187`; `single_pass_adapter_accepts_normalized_words_and_rejects_any_overlap` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:190-211`; `single_pass_punctuation_boundary_keeps_original_words_with_positive_ranges` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:214-230`; `vad_test_window` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:232-241`; `vad_test_word` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:243-249`; `vad_sentence_overlap_within_one_frame_stays_in_one_valid_segment` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:252-272`; `vad_overlapped_sentence_word_extends_group_to_later_end` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:275-293`; `vad_sentence_overlap_over_one_tdt_frame_is_rejected` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:296-312`; `gpu_attestation_ack_is_bound_to_the_current_session` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:315-338`; `missing_cuda_worker_is_reported_without_changing_cpu_path` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:341-347`; `compile_fake_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:349-444`; `compile_multi_session_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:446-507`; `receive_worker_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:509-521`; `retained_import_command` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:523-539`; `retained_worker_reads_two_correlated_import_sessions_and_hits_model_cache` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:542-580`; `failed_publish_drops_pending_worker_without_starting_retention` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:583-623`; `compile_verified_gpu_init_failure_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:625-689`; `poll_until` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:691-719`; `prepared_import` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:721-727`; `fake_gpu_candidate` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:729-739`; `stop_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:741-748`; `auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:751-785`; `auto_cpu_startup_failure_ends_without_another_attempt` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:788-818`; `auto_gpu_ready_is_attested_and_explicitly_admitted` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:821-863`; `auto_import_gpu_exit_after_ready_before_begin_retries_once_on_cpu` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:866-905`; `auto_import_unlaunchable_gpu_uses_one_cpu_attempt` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:908-929`; `gpu_import_failed_then_eof_retries_cpu_and_accepts_same_job_ready` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:932-977`; `gpu_live_failed_then_eof_retries_cpu_and_accepts_ready_before_capture_activation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:980-1044`; `forced_gpu_initialization_failure_waits_for_explicit_cpu_choice` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1047-1090`; `stop_latched_after_gpu_failure_prevents_cpu_worker_launch` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1093-1116`; `auto_import_gpu_failure_retains_request_until_correlated_stop_for_cpu_retry` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1119-1215`; `auto_import_does_not_retry_cpu_after_gpu_ready` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1218-1248`; `queued_import_record_accepts_legacy_live_offset_field` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1251-1276`; `pending_reservation_rejects_three_event_burst_at_twenty_slots` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1279-1287`; `stale_mp3_and_failed_events_have_old_attempt_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1290-1311`; `failed_worker_message_does_not_hide_later_correlated_stop` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1314-1338`; `stale_stopped_does_not_hide_later_correlated_stop` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1341-1365`; `worker_reader_signals_transport_end_after_session_end` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1368-1391`; `slow_worker_backpressure_keeps_command_queue_bounded_and_retries_window` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1394-1417`; `capture_failure_stop_is_retained_when_data_channel_is_full` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1420-1449`; `stdin_frames_preserve_live_start_and_admitted_window_before_normal_stop` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1452-1520`; `stopped_worker_waits_until_live_segment_persistence_is_acknowledged` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1523-1534`; `replay_windows_map_after_confirmed_prefix_without_duplicate_coverage` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1537-1542`; `queued_live_start_is_applied_before_an_immediate_stop` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1545-1603`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1548-1564`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1566-1568`; `gpu_choice_keeps_drained_counters_ahead_of_following_events` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1606-1638`; `stop_counters_reach_application_before_final_window` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1641-1723`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1647-1663`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1665-1679`; `direct_micro_start_and_single_stop_are_applied_in_order` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1726-1784`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1729-1738`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1740-1742`; `saturated_pending_queue_cannot_start_a_live_job_after_stop_and_allows_next_start` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1787-1922`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1795-1810`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1812-1823`; `stop_before_commit_atomically_prevents_publication` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1925-1938`; `cancelled_gate_skips_live_start_before_any_worker_or_capture_side_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1941-1976`; `effect_queue_accepts_eight_and_rejects_the_ninth_without_blocking` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1979-1996`; `persist_segment_queue_full_is_distinct_from_disconnection` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1999-2040`; `event_65_backpressures_all_production_stages_then_delivers_in_order` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2043-2194`; `apply_effect` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2050`; `poll_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2052-2095`; `stop_control_is_admitted_when_effect_queue_is_full` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2197-2214`; `frame_limit_counts_newline_and_rejects_oversize_before_write` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2217-2246`; `framed_ipc_rejects_incomplete_oversized_and_v1_messages` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2249-2261`; `worker_eof_without_end_is_reported_as_failure` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2264-2270`; `test_destination` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2272-2276`; `test_request` [tests] `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:2278-2296`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1-2296`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1-2296`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs`

Tests ou supports : définit `r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication` `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:404-424`, `queued_request_identity_allows_only_compute_choice_and_verified_model_path_binding` `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:427-444`.

Mesure : 0 production / 1401 tests / 1401 physiques. `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1-1401`

Types : `NextGeneration` (trait, tests) `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:391-393`.

Fonctions avec corps : `compile_copy_resume_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1-47`; `write_copy_resume_wav` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:49-66`; `copy_resume_request` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:68-90`; `copy_resume_hex` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:92-94`; `copy_resume_generation_path` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:96-100`; `wait_copy_resume_prompt` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:102-118`; `run_integrated_copy_resume_case` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:120-389`; `clone_with_next_generation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:396-400`; `r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:404-424`; `queued_request_identity_allows_only_compute_choice_and_verified_model_path_binding` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:427-444`; `queued_manual_gpu_preflight_requires_a_current_candidate_and_gpu_key` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:447-477`; `queued_manual_gpu_without_current_proof_refuses_before_pending_creation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:480-554`; `queued_unified_gpu_refusal_preserves_durable_entry_and_source_before_preparation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:557-688`; `queued_unified_non_candidate_refusal_does_not_prepare_or_start` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:691-804`; `queued_unified_capacity_refusal_keeps_fifo_entry_without_preparing_route` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:807-940`; `l3c_acceptance_fault_is_confined_to_its_canonical_execution_root` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:944-995`; `over_limit_parakeet_import_is_refused_before_pending_creation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:998-1061`; `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1064-1153`; `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1156-1271`; `hex_sha256` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1273-1286`; `stop_during_blocked_commit_returns_promptly_and_reports_finalizing` [tests] `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1289-1401`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1-1401`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1-1401`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc — `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs`

Tests ou supports : définit `observe_controlled_child` `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:140-164`.

Mesure : 0 production / 189 tests / 189 physiques. `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:1-189`

Types : `NativeQualificationEventKind` (enum, tests) `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:3-61`; `NativeQualificationWork` (enum, tests) `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:65-68`; `NativeQualificationEvent` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:72-75`; `NativeQualificationObserver` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:78-82`; `ControlledChildObservation` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:129-137`; `ScheduledWindowTranscriber` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:167-171`.

Fonctions avec corps : `channel` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:86-96`; `allocate_incarnation` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:98-104`; `record` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:106-117`; `assert_healthy` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:119-124`; `observe_controlled_child` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:140-164`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:175-177`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:179-188`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:1-189`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs:1-189`.

Utilisé par, via ces imports : hors tests : `crate::direct_live` `crates/whisper-adapters/src/direct_live.rs:42`.

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs`

Tests ou supports : définit `existing_source_binding_is_reused_and_failed_acquisition_is_not_inserted` `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:2-61`.

Mesure : 0 production / 61 tests / 61 physiques. `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:1-61`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:1-61`.

Fonctions avec corps : `existing_source_binding_is_reused_and_failed_acquisition_is_not_inserted` [tests] `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:2-61`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:1-61`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/source_binding_entry.rs:1-61`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc — `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs`

Tests ou supports : définit `stopped_ack_waits_for_exit_then_allows_completion` `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:6-16`.

Mesure : 0 production / 17 tests / 17 physiques. `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`.

Fonctions avec corps : `stopped_ack_waits_for_exit_then_allows_completion` [tests] `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:6-16`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `parakeet_stopped_ack_tests`

Tests/auxiliaires : `stopped_ack_waits_for_exit_then_allows_completion` `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:6-16`. `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`

Mesure locale : 0 production / 17 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`

Types locaux : aucun relevé dans `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/src/worker_ipc/tests/stopped_ack.rs:1-17`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs`

Tests ou supports : définit `import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio` `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2-390`, `global_close_waits_for_both_capture_publications_and_refuses_new_admission` `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:393-625`.

Mesure : 0 production / 3034 tests / 3034 physiques. `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1-3034`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1-3034`.

Fonctions avec corps : `import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2-390`; `global_close_waits_for_both_capture_publications_and_refuses_new_admission` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:393-625`; `import_application_import_a_and_capture_b_share_one_held_backend_until_b_is_published` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:628-835`; `unified_capture_updates_preview_and_final_document_before_stop_then_publishes_the_tail` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:838-1024`; `controlled_runtime_recycles_scheduler_slots_across_many_unified_imports` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1027-1053`; `import_application_decodes_real_mp3_and_publishes_full_unified_coverage` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1056-1215`; `controlled_bounded_gpu_prepare_uses_identity_without_legacy_session_qualification` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1218-1249`; `controlled_document_drain_reads_one_segment_per_poll_and_preserves_all_pages` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1252-1301`; `controlled_document_drain_terminates_after_page_read_error` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1304-1351`; `unified_terminal_failures_release_all_parent_state_and_preserve_source_audio` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1354-1450`; `unrecycled_controlled_child_rejects_third_job_with_bounded_table` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1453-1564`; `unified_document_finalization_failure_releases_session_and_allows_replacement` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1567-1684`; `capture_identity_does_not_mask_existing_import_source_binding` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1687-1758`; `capture_with_empty_snapshot_registry_publishes_complete_source_without_registry_state` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1761-1918`; `capture_identity_with_orphaned_snapshot_artifact_fails_closed` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1921-1973`; `capture_identity_does_not_mask_corrupt_snapshot_registry` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1976-2020`; `unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2023-2472`; `unified_child_retains_only_two_job_ids_until_real_worker_recycle` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2475-2536`; `panicking_unified_worker_terminates_the_inflight_session_and_its_peer` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2539-2627`; `unified_document_drains_are_included_in_the_two_session_admission_bound` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2630-2691`; `disconnected_unified_inference_result_fails_the_inflight_session_and_preserves_its_peer` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2694-2777`; `unified_capture_gain_changes_apply_to_the_next_written_pcm_block_and_reject_stale_targets` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2780-2932`; `unified_import_and_capture_without_controlled_injection_are_refused_before_hardware_start` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2935-3034`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1-3034`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1-3034`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs`

Tests ou supports : définit `unified_capture_runtime_rejects_third_before_capture_factory` `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:2-96`, `native_unified_capture_open_failure_leaves_no_session_or_capacity` `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:99-135`.

Mesure : 0 production / 2566 tests / 2566 physiques. `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1-2566`

Types : `Artifact` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:150-154`; `Manifest` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:157-165`; `Artifact` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:262-266`; `Manifest` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:269-277`; `PublicationIntent` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:280-287`.

Fonctions avec corps : `unified_capture_runtime_rejects_third_before_capture_factory` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:2-96`; `native_unified_capture_open_failure_leaves_no_session_or_capacity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:99-135`; `assert_real_unified_model` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:137-143`; `assert_app_view_internal_publication` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:145-252`; `assert_app_view_external_publication` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:254-463`; `enqueue_import_with_choice` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:465-502`; `process_queued_import_to_app_view` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:504-539`; `import_application_uses_real_archive_path_for_controlled_short_and_long_unified_imports` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:542-714`; `import_application_processes_durable_queued_unified_imports_for_cpu_gpu_auto_and_long_audio` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:717-869`; `queued_create_only_publication_paths_reach_app_view` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:872-973`; `resumed_create_only_publication_paths_reach_app_view` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:976-1405`; `reprocessing_create_only_publication_paths_reach_app_view_and_preserve_earlier_result` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1409-1559`; `import_application_resumes_a_verified_interrupted_unified_import` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1562-1964`; `failed_unified_resume_preserves_interrupted_source_binding_and_queue_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1967-2119`; `failed_unified_resume_after_binding_transfer_restores_registry_and_queue_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:2122-2364`; `controlled_import_publishes_audio_over_one_gibibyte_across_physical_parts` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:2369-2566`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1-2566`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1-2566`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs`

Tests ou supports : définit `unified_capture_after_direct_stop_uses_its_own_gate_and_publishes_audio` `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:87-364`, `unified_start_and_immediate_stop_keep_the_new_session_identity` `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:367-418`.

Mesure : 0 production / 564 tests / 564 physiques. `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:1-564`

Types : `ServiceRuntimeTestPort` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:1-8`.

Fonctions avec corps : `submit` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:11-20`; `poll` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:22-39`; `install_foreign_corrupt_archive` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:41-85`; `unified_capture_after_direct_stop_uses_its_own_gate_and_publishes_audio` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:87-364`; `unified_start_and_immediate_stop_keep_the_new_session_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:367-418`; `unified_capture_alone_routes_probe_and_rejects_old_or_wrong_source_events` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:421-539`; `corrupt_archive_alias_cannot_borrow_canonical_foreign_identity` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:542-564`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:1-564`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:1-564`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs`

Tests ou supports : définit `recette_policy_l2a2_requires_the_feature` `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:13-23`.

Mesure : 0 production / 656 tests / 656 physiques. `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:1-656`

Types : `RuntimeImportPort` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:176-183`; `ControlledUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:271-273`; `StartFailsOnceUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:297-300`; `HeldUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:342-351`; `TailAwareUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:460`; `ObservedTailAwareUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:487-489`; `InvalidTimeUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:514`; `FailedUnifiedWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:533`; `ExportCollisionWorker` (struct, tests) `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:548-551`.

Fonctions avec corps : `recette_policy_l2a2_requires_the_feature` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:13-23`; `record_import_port_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:29-38`; `controlled_child_job_bound` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:40-54`; `persist_and_publish_controlled_document_event` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:56-133`; `prepare_and_start_controlled_native_route` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:135-174`; `submit` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:186-207`; `poll` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:209-268`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:276-278`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:280-294`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:303-305`; `initialize_instance` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:307-313`; `stop_instance` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:315-318`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:320-326`; `controlled_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:329-340`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:354-356`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:358-397`; `held_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:400-411`; `held_final_a_worker` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:413-422`; `held_worker_with_mode` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:424-458`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:463-465`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:467-484`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:492-494`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:496-511`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:517-519`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:520-530`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:536-538`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:539-545`; `instance_id` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:554-556`; `transcribe` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:557-573`; `unified_import_request` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:576-607`; `unified_live_request` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:609-640`; `controlled_native_route` [tests] `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:642-656`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:1-656`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:1-656`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs`

Tests ou supports : définit `worker_command_poll_prioritizes_control_and_distinguishes_idle_and_closed` `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:2-40`.

Mesure : 0 production / 40 tests / 40 physiques. `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:1-40`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:1-40`.

Fonctions avec corps : `worker_command_poll_prioritizes_control_and_distinguishes_idle_and_closed` [tests] `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:2-40`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:1-40`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests/worker_command_poll.rs:1-40`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc::tests — `crates/whisper-adapters/src/worker_ipc/tests.rs`

Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`.

Mesure : 0 production / 14 tests / 14 physiques. `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`.

Fonctions avec corps : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/src/worker_ipc/tests.rs:1-14`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker_ipc — `crates/whisper-adapters/src/worker_ipc.rs`

Réexporte transport/service et rassemble états et modules du runtime IPC. `crates/whisper-adapters/src/worker_ipc.rs:1-140`

Mesure : 85 production / 55 tests / 140 physiques. `crates/whisper-adapters/src/worker_ipc.rs:1-140`

Types : `PublicationGateRegistry` (type, hors tests) `crates/whisper-adapters/src/worker_ipc.rs:122`; `WorkerRuntimeConfig` (struct, hors tests) `crates/whisper-adapters/src/worker_ipc.rs:125-131`.

Fonctions avec corps : aucune relevée dans `crates/whisper-adapters/src/worker_ipc.rs:1-140`.

Déclarations `pub` (visibilité sous conditions) : `pub use backends::{probe_cuda_worker, qualify_gpu_worker};` `crates/whisper-adapters/src/worker_ipc.rs:39`; `pub use ipc::ChildWorkerTransport;` `crates/whisper-adapters/src/worker_ipc.rs:53`; `pub use service::AsyncImportIo;` `crates/whisper-adapters/src/worker_ipc.rs:60`.

Dépend de, via `use crate::` : hors tests : `crate::WorkerTransport` → [Non vérifié] symbole/réexport non résolu `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::archive::ArchiveStore` → `crate::archive` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::archive::LiveArchive` → `crate::archive` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::archive` → `crate::archive` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::capture::CaptureStream` → `crate::capture` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::capture::Pcm16Converter` → `crate::capture` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::capture` → `crate::capture` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::staging::PcmStaging` → `crate::staging` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::staging::PcmWriter` → `crate::staging` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::staging` → `crate::staging` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::DiagnosticCode` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::DiagnosticPhase` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::DiagnosticRecord` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::DiagnosticSink` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::ProgressPhase` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::ProgressSupervisor` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor::WorkerPaths` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::supervisor` → `crate::supervisor` `crates/whisper-adapters/src/worker_ipc.rs:79`<br>hors tests : `crate::vad::VoiceActivity` → `crate::vad` `crates/whisper-adapters/src/worker_ipc.rs:79`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::decoder_proxy — `crates/whisper-adapters/tests/decoder_proxy.rs`

Tests ou supports : définit `decoder_proxy_rejects_a_stale_generation_before_returning_audio` `crates/whisper-adapters/tests/decoder_proxy.rs:55-72`, `decoder_proxy_accepts_only_matching_source_job_generation_and_range` `crates/whisper-adapters/tests/decoder_proxy.rs:75-85`.

Mesure : 0 production / 111 tests / 111 physiques. `crates/whisper-adapters/tests/decoder_proxy.rs:1-111`

Types : `StubTransport` (struct, tests) `crates/whisper-adapters/tests/decoder_proxy.rs:8-11`.

Fonctions avec corps : `request` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:14-17`; `receive` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:19-21`; `request` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:24-35`; `response` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:37-52`; `decoder_proxy_rejects_a_stale_generation_before_returning_audio` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:55-72`; `decoder_proxy_accepts_only_matching_source_job_generation_and_range` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:75-85`; `decoder_proxy_accepts_bounded_partial_range_and_rejects_out_of_range_block` [tests] `crates/whisper-adapters/tests/decoder_proxy.rs:88-111`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/decoder_proxy.rs:1-111`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/decoder_proxy.rs:1-111`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::durability — `crates/whisper-adapters/tests/durability.rs`

Tests ou supports : définit `queue_ack_replays_fifo_and_running_work_recovers_without_launch` `crates/whisper-adapters/tests/durability.rs:30-125`, `resume_validation_rejects_later_fifo_row_without_advancing_any_generation` `crates/whisper-adapters/tests/durability.rs:128-183`.

Mesure : 0 production / 183 tests / 183 physiques. `crates/whisper-adapters/tests/durability.rs:1-183`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/durability.rs:1-183`.

Fonctions avec corps : `request` [tests] `crates/whisper-adapters/tests/durability.rs:11-27`; `queue_ack_replays_fifo_and_running_work_recovers_without_launch` [tests] `crates/whisper-adapters/tests/durability.rs:30-125`; `resume_validation_rejects_later_fifo_row_without_advancing_any_generation` [tests] `crates/whisper-adapters/tests/durability.rs:128-183`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/durability.rs:1-183`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/durability.rs:1-183`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_inventory — `crates/whisper-adapters/tests/gpu_inventory.rs`

Tests ou supports : définit `maps_integrated_gpu_as_current_without_cuda_eligibility_inference` `crates/whisper-adapters/tests/gpu_inventory.rs:5-15`, `maps_disabled_present_card_and_preserves_ambiguous_windows_error_codes` `crates/whisper-adapters/tests/gpu_inventory.rs:18-26`.

Mesure : 0 production / 44 tests / 44 physiques. `crates/whisper-adapters/tests/gpu_inventory.rs:1-44`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/gpu_inventory.rs:1-44`.

Fonctions avec corps : `maps_integrated_gpu_as_current_without_cuda_eligibility_inference` [tests] `crates/whisper-adapters/tests/gpu_inventory.rs:5-15`; `maps_disabled_present_card_and_preserves_ambiguous_windows_error_codes` [tests] `crates/whisper-adapters/tests/gpu_inventory.rs:18-26`; `missing_presence_is_unknown_and_false_presence_is_absent` [tests] `crates/whisper-adapters/tests/gpu_inventory.rs:29-38`; `missing_identity_cannot_be_claimed_as_a_current_device` [tests] `crates/whisper-adapters/tests/gpu_inventory.rs:41-44`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/gpu_inventory.rs:1-44`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/gpu_inventory.rs:1-44`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_qualification_transport — `crates/whisper-adapters/tests/gpu_qualification_transport.rs`

Tests ou supports : définit `qualification_response_is_typed_and_keeps_per_device_identity` `crates/whisper-adapters/tests/gpu_qualification_transport.rs:4-24`.

Mesure : 0 production / 24 tests / 24 physiques. `crates/whisper-adapters/tests/gpu_qualification_transport.rs:1-24`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/gpu_qualification_transport.rs:1-24`.

Fonctions avec corps : `qualification_response_is_typed_and_keeps_per_device_identity` [tests] `crates/whisper-adapters/tests/gpu_qualification_transport.rs:4-24`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/gpu_qualification_transport.rs:1-24`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/gpu_qualification_transport.rs:1-24`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_selection_worker — `crates/whisper-adapters/tests/gpu_selection_worker.rs`

Tests ou supports : définit `gpu_import_and_live_commands_keep_candidate_identity_and_distinct_ordinals` `crates/whisper-adapters/tests/gpu_selection_worker.rs:23-82`, `real_gpu_import_uses_current_windows_candidate_and_local_model_when_enabled` `crates/whisper-adapters/tests/gpu_selection_worker.rs:86-290`.

Mesure : 0 production / 336 tests / 336 physiques. `crates/whisper-adapters/tests/gpu_selection_worker.rs:1-336`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/gpu_selection_worker.rs:1-336`.

Fonctions avec corps : `candidate` [tests] `crates/whisper-adapters/tests/gpu_selection_worker.rs:10-20`; `gpu_import_and_live_commands_keep_candidate_identity_and_distinct_ordinals` [tests] `crates/whisper-adapters/tests/gpu_selection_worker.rs:23-82`; `real_gpu_import_uses_current_windows_candidate_and_local_model_when_enabled` [tests] `crates/whisper-adapters/tests/gpu_selection_worker.rs:86-290`; `wav_pcm_samples_at_16khz` [tests] `crates/whisper-adapters/tests/gpu_selection_worker.rs:293-320`; `sha256_file` [tests] `crates/whisper-adapters/tests/gpu_selection_worker.rs:323-336`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/gpu_selection_worker.rs:1-336`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/gpu_selection_worker.rs:1-336`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_settings — `crates/whisper-adapters/tests/gpu_settings.rs`

Tests ou supports : définit `old_json_migrates_additively_and_reopens_with_revision_acknowledgement` `crates/whisper-adapters/tests/gpu_settings.rs:62-100`, `temporary_substitution_uses_q_then_restores_preferred_p` `crates/whisper-adapters/tests/gpu_settings.rs:103-138`.

Mesure : 0 production / 203 tests / 203 physiques. `crates/whisper-adapters/tests/gpu_settings.rs:1-203`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/gpu_settings.rs:1-203`.

Fonctions avec corps : `isolated_dir` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:19-29`; `defaults` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:31-37`; `wait_for_latest_save` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:39-59`; `old_json_migrates_additively_and_reopens_with_revision_acknowledgement` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:62-100`; `temporary_substitution_uses_q_then_restores_preferred_p` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:103-138`; `interleaved_path_microphone_and_gpu_edits_preserve_one_latest_snapshot` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:141-178`; `write_failure_is_attributed_to_revision_and_reported` [tests] `crates/whisper-adapters/tests/gpu_settings.rs:181-203`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/gpu_settings.rs:1-203`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/gpu_settings.rs:1-203`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::import_cpu — `crates/whisper-adapters/tests/import_cpu.rs`

Tests ou supports : définit `r11_duplicate_import_prompts_before_queue_and_replace_keeps_all_old_archives` `crates/whisper-adapters/tests/import_cpu.rs:26-345`, `real_wav_runs_through_cpu_worker_and_verified_archive` `crates/whisper-adapters/tests/import_cpu.rs:1290-1409`.

Mesure : 0 production / 2249 tests / 2249 physiques. `crates/whisper-adapters/tests/import_cpu.rs:1-2249`

Types : `NoCallTransport` (struct, tests) `crates/whisper-adapters/tests/import_cpu.rs:1734`; `MemoryObservation` (struct, tests) `crates/whisper-adapters/tests/import_cpu.rs:1775-1782`; `MemoryObservation` (struct, tests) `crates/whisper-adapters/tests/import_cpu.rs:2222`.

Fonctions avec corps : `r11_duplicate_import_prompts_before_queue_and_replace_keeps_all_old_archives` [tests] `crates/whisper-adapters/tests/import_cpu.rs:26-345`; `r11_collision_reprompts_before_queue` [tests] `crates/whisper-adapters/tests/import_cpu.rs:347-409`; `r11_resume_keeps_replace_reservation` [tests] `crates/whisper-adapters/tests/import_cpu.rs:411-542`; `r11_replace_with_missing_old_exports_and_third_party_name` [tests] `crates/whisper-adapters/tests/import_cpu.rs:544-637`; `r11_byte_identical_post_reservation_collision_is_never_claimed` [tests] `crates/whisper-adapters/tests/import_cpu.rs:639-763`; `r11_exports_recover_after_one_or_two_export_interruption` [tests] `crates/whisper-adapters/tests/import_cpu.rs:765-772`; `r11_editing_visible_export_preserves_internal_generation` [tests] `crates/whisper-adapters/tests/import_cpu.rs:774-826`; `r11_recover_after_single_export_collision` [tests] `crates/whisper-adapters/tests/import_cpu.rs:828-958`; `r11_recover_after_both_exports_before_current` [tests] `crates/whisper-adapters/tests/import_cpu.rs:960-1100`; `r11_reservation_path` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1102-1110`; `r11_generation_path` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1112-1121`; `r11_read_publication_intent` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1123-1136`; `publish_r11_fixture` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1138-1164`; `publish_r11_fixture_reserved` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1166-1182`; `r11_request` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1184-1205`; `wait_r11_prompt` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1207-1225`; `wait_r11_queue_entry` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1227-1247`; `wait_r11_queue_empty` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1249-1261`; `write_r11_silence_wav` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1263-1280`; `unix_nanos` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1282-1287`; `real_wav_runs_through_cpu_worker_and_verified_archive` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1290-1409`; `generated_long_wav_runs_eight_full_native_windows_with_stage_memory_samples` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1413-1515`; `write_generated_silence_wav` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1518-1537`; `stop_during_native_inference_is_prompt_and_never_publishes_complete` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1540-1659`; `worker_cpu_time` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1661-1676`; `real_worker_serves_correlated_bounded_decode_blocks` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1679-1730`; `decoder_rejects_invalid_window_bounds_before_transport_admission` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1733-1772`; `request` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1736-1739`; `receive` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1740-1742`; `start` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1786-1901`; `finish` [tests] `crates/whisper-adapters/tests/import_cpu.rs:1903-2196`; `process_memory_for` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2200-2208`; `drop` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2212-2218`; `start` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2226-2228`; `finish` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2230`; `parse_hash` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2233-2239`; `digit` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2240-2246`; `hex` [tests] `crates/whisper-adapters/tests/import_cpu.rs:2247-2249`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/import_cpu.rs:1-2249`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/import_cpu.rs:1-2249`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::import_mp3 — `crates/whisper-adapters/tests/import_mp3.rs`

Tests ou supports : définit `r11_mp3_content_identity_is_independent_of_filename_and_destination` `crates/whisper-adapters/tests/import_mp3.rs:12-118`, `source_changed_after_queue_admission_is_refused_without_worker_launch` `crates/whisper-adapters/tests/import_mp3.rs:169-264`.

Mesure : 0 production / 471 tests / 471 physiques. `crates/whisper-adapters/tests/import_mp3.rs:1-471`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/import_mp3.rs:1-471`.

Fonctions avec corps : `r11_mp3_content_identity_is_independent_of_filename_and_destination` [tests] `crates/whisper-adapters/tests/import_mp3.rs:12-118`; `r11_mp3_request` [tests] `crates/whisper-adapters/tests/import_mp3.rs:120-145`; `wait_r11_mp3_prompt` [tests] `crates/whisper-adapters/tests/import_mp3.rs:147-162`; `hex` [tests] `crates/whisper-adapters/tests/import_mp3.rs:164-166`; `source_changed_after_queue_admission_is_refused_without_worker_launch` [tests] `crates/whisper-adapters/tests/import_mp3.rs:169-264`; `failed_preparation_keeps_durable_row_actionable` [tests] `crates/whisper-adapters/tests/import_mp3.rs:267-364`; `real_mp3_runs_through_durable_choice_cpu_worker_and_txt_srt_archive` [tests] `crates/whisper-adapters/tests/import_mp3.rs:367-471`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/import_mp3.rs:1-471`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/import_mp3.rs:1-471`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::legacy_model_requests — `crates/whisper-adapters/tests/legacy_model_requests.rs`

Tests ou supports : définit `queued_legacy_turbo_path_is_resolved_in_memory_and_durable_record_is_unchanged` `crates/whisper-adapters/tests/legacy_model_requests.rs:72-96`, `unknown_queued_path_is_refused_with_an_explicit_diagnostic` `crates/whisper-adapters/tests/legacy_model_requests.rs:99-118`.

Mesure : 0 production / 163 tests / 163 physiques. `crates/whisper-adapters/tests/legacy_model_requests.rs:1-163`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/legacy_model_requests.rs:1-163`.

Fonctions avec corps : `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:14-28`; `root` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:33-36`; `request` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:38-61`; `verified_catalog` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:63-69`; `queued_legacy_turbo_path_is_resolved_in_memory_and_durable_record_is_unchanged` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:72-96`; `unknown_queued_path_is_refused_with_an_explicit_diagnostic` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:99-118`; `explicit_resume_accepts_a_catalog_path_for_known_turbo_and_keeps_durable_path` [tests] `crates/whisper-adapters/tests/legacy_model_requests.rs:121-163`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/legacy_model_requests.rs:1-163`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/legacy_model_requests.rs:1-163`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::live_archive — `crates/whisper-adapters/tests/live_archive.rs`

Tests ou supports : définit `attempt_journal_reserves_monotonically_and_enforces_transitions` `crates/whisper-adapters/tests/live_archive.rs:20-66`, `a_live_group_has_one_archive_writer_at_a_time` `crates/whisper-adapters/tests/live_archive.rs:69-94`.

Mesure : 0 production / 847 tests / 847 physiques. `crates/whisper-adapters/tests/live_archive.rs:1-847`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/live_archive.rs:1-847`.

Fonctions avec corps : `test_root` [tests] `crates/whisper-adapters/tests/live_archive.rs:8-17`; `attempt_journal_reserves_monotonically_and_enforces_transitions` [tests] `crates/whisper-adapters/tests/live_archive.rs:20-66`; `a_live_group_has_one_archive_writer_at_a_time` [tests] `crates/whisper-adapters/tests/live_archive.rs:69-94`; `next_passage_refuses_durable_pcm_without_verified_coverage` [tests] `crates/whisper-adapters/tests/live_archive.rs:97-135`; `fully_covered_but_unpublished_passage_stays_recoverable_and_blocks_next_generation` [tests] `crates/whisper-adapters/tests/live_archive.rs:138-202`; `next_passage_refuses_text_after_the_last_coverage_commit` [tests] `crates/whisper-adapters/tests/live_archive.rs:205-263`; `next_passage_refuses_unconfirmed_pcm_after_the_durable_checkpoint` [tests] `crates/whisper-adapters/tests/live_archive.rs:266-325`; `passage_publishes_durable_pcm_mp3_and_confirmed_text_as_one_generation` [tests] `crates/whisper-adapters/tests/live_archive.rs:328-564`; `recovery_adopts_a_synced_silent_window_commit_without_a_durable_ack` [tests] `crates/whisper-adapters/tests/live_archive.rs:567-620`; `characterization_live_request` [tests] `crates/whisper-adapters/tests/live_archive.rs:622-638`; `characterization_live_dir` [tests] `crates/whisper-adapters/tests/live_archive.rs:640-644`; `archive_characterization_attempt_journal_corruption_is_not_read_by_recovery` [tests] `crates/whisper-adapters/tests/live_archive.rs:647-698`; `archive_characterization_live_coverage_checksum_and_order_are_refused` [tests] `crates/whisper-adapters/tests/live_archive.rs:701-743`; `archive_characterization_live_segment_before_coverage_stays_unconfirmed` [tests] `crates/whisper-adapters/tests/live_archive.rs:746-784`; `archive_characterization_live_current_manifest_and_snapshot_corruption_are_stable` [tests] `crates/whisper-adapters/tests/live_archive.rs:787-847`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/live_archive.rs:1-847`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/live_archive.rs:1-847`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::live_capture — `crates/whisper-adapters/tests/live_capture.rs`

Tests ou supports : définit `capture_conversion_rejects_unsupported_dimensions_and_uses_bounded_slots` `crates/whisper-adapters/tests/live_capture.rs:8-13`, `bounded_pcm_writer_drains_queued_frames_to_a_verified_checkpoint` `crates/whisper-adapters/tests/live_capture.rs:16-45`.

Mesure : 0 production / 78 tests / 78 physiques. `crates/whisper-adapters/tests/live_capture.rs:1-78`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/live_capture.rs:1-78`.

Fonctions avec corps : `capture_conversion_rejects_unsupported_dimensions_and_uses_bounded_slots` [tests] `crates/whisper-adapters/tests/live_capture.rs:8-13`; `bounded_pcm_writer_drains_queued_frames_to_a_verified_checkpoint` [tests] `crates/whisper-adapters/tests/live_capture.rs:16-45`; `delayed_durable_high_water_keeps_the_80k_inference_window_on_disk` [tests] `crates/whisper-adapters/tests/live_capture.rs:48-78`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/live_capture.rs:1-78`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/live_capture.rs:1-78`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::microphone_level — `crates/whisper-adapters/tests/microphone_level.rs`

Tests ou supports : définit `capture_path_accepts_the_declared_microphone_gain_range_and_bounded_queue` `crates/whisper-adapters/tests/microphone_level.rs:5-12`.

Mesure : 0 production / 12 tests / 12 physiques. `crates/whisper-adapters/tests/microphone_level.rs:1-12`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/microphone_level.rs:1-12`.

Fonctions avec corps : `capture_path_accepts_the_declared_microphone_gain_range_and_bounded_queue` [tests] `crates/whisper-adapters/tests/microphone_level.rs:5-12`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/microphone_level.rs:1-12`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/microphone_level.rs:1-12`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::microphone_probe — `crates/whisper-adapters/tests/microphone_probe.rs`

Tests ou supports : définit `analyzer_publishes_bounded_psd_and_slow_readers_keep_immutable_snapshots` `crates/whisper-adapters/tests/microphone_probe.rs:60-88`, `invalidation_hides_a_snapshot_even_when_a_reader_still_holds_it` `crates/whisper-adapters/tests/microphone_probe.rs:91-104`.

Mesure : 0 production / 203 tests / 203 physiques. `crates/whisper-adapters/tests/microphone_probe.rs:1-203`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/microphone_probe.rs:1-203`.

Fonctions avec corps : `block` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:17-37`; `wait_snapshot` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:39-50`; `submit_blocks` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:52-57`; `analyzer_publishes_bounded_psd_and_slow_readers_keep_immutable_snapshots` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:60-88`; `invalidation_hides_a_snapshot_even_when_a_reader_still_holds_it` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:91-104`; `real_queue_overflow_is_counted_and_measurement_recovers_after_a_full_window` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:107-126`; `spectrum_is_cached_across_rms_ticks_and_invalidated_once_on_loss` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:129-161`; `fft_peak_tracks_hz_and_gain_window_records_return_to_initial_value` [tests] `crates/whisper-adapters/tests/microphone_probe.rs:164-203`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/microphone_probe.rs:1-203`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/microphone_probe.rs:1-203`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::microphone_settings — `crates/whisper-adapters/tests/microphone_settings.rs`

Tests ou supports : définit `old_settings_default_to_windows_microphone_without_losing_paths` `crates/whisper-adapters/tests/microphone_settings.rs:4-11`, `microphone_id_is_kept_in_the_same_settings_snapshot` `crates/whisper-adapters/tests/microphone_settings.rs:14-27`.

Mesure : 0 production / 27 tests / 27 physiques. `crates/whisper-adapters/tests/microphone_settings.rs:1-27`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/microphone_settings.rs:1-27`.

Fonctions avec corps : `old_settings_default_to_windows_microphone_without_losing_paths` [tests] `crates/whisper-adapters/tests/microphone_settings.rs:4-11`; `microphone_id_is_kept_in_the_same_settings_snapshot` [tests] `crates/whisper-adapters/tests/microphone_settings.rs:14-27`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/microphone_settings.rs:1-27`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/microphone_settings.rs:1-27`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::model_catalog — `crates/whisper-adapters/tests/model_catalog.rs`

Tests ou supports : définit `closed_catalog_reports_both_pinned_model_files_and_exact_observations` `crates/whisper-adapters/tests/model_catalog.rs:66-88`, `missing_and_corrupt_models_are_reported_without_touching_the_catalog_root` `crates/whisper-adapters/tests/model_catalog.rs:91-121`.

Mesure : 0 production / 231 tests / 231 physiques. `crates/whisper-adapters/tests/model_catalog.rs:1-231`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/model_catalog.rs:1-231`.

Fonctions avec corps : `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/tests/model_catalog.rs:11-25`; `models_root` [tests] `crates/whisper-adapters/tests/model_catalog.rs:27-29`; `runtime_root` [tests] `crates/whisper-adapters/tests/model_catalog.rs:31-34`; `cache_test_path` [tests] `crates/whisper-adapters/tests/model_catalog.rs:36-40`; `turbo_request` [tests] `crates/whisper-adapters/tests/model_catalog.rs:42-63`; `closed_catalog_reports_both_pinned_model_files_and_exact_observations` [tests] `crates/whisper-adapters/tests/model_catalog.rs:66-88`; `missing_and_corrupt_models_are_reported_without_touching_the_catalog_root` [tests] `crates/whisper-adapters/tests/model_catalog.rs:91-121`; `known_historical_turbo_path_rebinds_to_the_verified_test_catalog_path` [tests] `crates/whisper-adapters/tests/model_catalog.rs:124-150`; `model_hash_cache_reuses_physical_identity_and_rehashes_changed_fingerprints` [tests] `crates/whisper-adapters/tests/model_catalog.rs:153-195`; `model_hash_cache_serializes_concurrent_first_reads_and_fails_closed_on_missing_files` [tests] `crates/whisper-adapters/tests/model_catalog.rs:198-231`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/model_catalog.rs:1-231`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/model_catalog.rs:1-231`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::model_preferences — `crates/whisper-adapters/tests/model_preferences.rs`

Tests ou supports : définit `v1_backup_is_created_before_v2_and_is_byte_identical_and_never_replaced` `crates/whisper-adapters/tests/model_preferences.rs:51-122`, `path_write_before_preference_migration_preserves_paths_and_v1_backup` `crates/whisper-adapters/tests/model_preferences.rs:125-163`.

Mesure : 0 production / 313 tests / 313 physiques. `crates/whisper-adapters/tests/model_preferences.rs:1-313`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/model_preferences.rs:1-313`.

Fonctions avec corps : `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/tests/model_preferences.rs:12-26`; `test_dir` [tests] `crates/whisper-adapters/tests/model_preferences.rs:28-34`; `v1_bytes` [tests] `crates/whisper-adapters/tests/model_preferences.rs:36-42`; `backup_path` [tests] `crates/whisper-adapters/tests/model_preferences.rs:44-48`; `v1_backup_is_created_before_v2_and_is_byte_identical_and_never_replaced` [tests] `crates/whisper-adapters/tests/model_preferences.rs:51-122`; `path_write_before_preference_migration_preserves_paths_and_v1_backup` [tests] `crates/whisper-adapters/tests/model_preferences.rs:125-163`; `concurrent_path_and_preference_writes_merge_after_v1_migration` [tests] `crates/whisper-adapters/tests/model_preferences.rs:166-210`; `wait_for_path_ack` [tests] `crates/whisper-adapters/tests/model_preferences.rs:212-226`; `conflicting_or_unwritable_backup_blocks_migration_and_preserves_v1` [tests] `crates/whisper-adapters/tests/model_preferences.rs:229-252`; `matching_backup_is_reused_once_and_backup_io_failure_leaves_v1_untouched` [tests] `crates/whisper-adapters/tests/model_preferences.rs:255-294`; `unknown_v1_path_is_retained_for_diagnostics_and_does_not_map_to_turbo` [tests] `crates/whisper-adapters/tests/model_preferences.rs:297-313`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/model_preferences.rs:1-313`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/model_preferences.rs:1-313`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_archive — `crates/whisper-adapters/tests/parakeet_archive.rs`

Tests ou supports : définit `import_strategy_defaults_to_legacy_and_round_trips_explicit_versions` `crates/whisper-adapters/tests/parakeet_archive.rs:25-60`, `parakeet_model_identity_prepares_persists_and_publishes_txt_srt` `crates/whisper-adapters/tests/parakeet_archive.rs:267-330`.

Mesure : 0 production / 2096 tests / 2096 physiques. `crates/whisper-adapters/tests/parakeet_archive.rs:1-2096`

Types : `TimedWord` (struct, tests) `crates/whisper-adapters/tests/parakeet_archive.rs:1480-1484`; `Inference` (struct, tests) `crates/whisper-adapters/tests/parakeet_archive.rs:1486-1491`; `ProductImport` (struct, tests) `crates/whisper-adapters/tests/parakeet_archive.rs:1791-1795`.

Fonctions avec corps : `import_strategy_defaults_to_legacy_and_round_trips_explicit_versions` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:25-60`; `required_path` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:62-64`; `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:66-80`; `not_run_without_l2_archive_inputs` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:82-89`; `archive_root` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:91-214`; `qualified_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:216-218`; `qualified_vad_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:220-225`; `qualified_l2b_single_pass_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:227-229`; `qualified_r4_single_pass_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:231-235`; `retired_l2a_single_pass_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:237-241`; `personal_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:243-264`; `parakeet_model_identity_prepares_persists_and_publishes_txt_srt` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:267-330`; `modified_parakeet_model_is_refused_before_pending_after_cache_revalidation` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:334-404`; `parakeet_archive_refuses_model_sha_divergence_before_pending_creation` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:407-442`; `pending_scan_and_resume_require_the_exact_qualified_test_key` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:445-608`; `registered_personal_pending_cpu_and_gpu_scan_and_resume_without_qualification_feature` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:611-699`; `r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:702-803`; `retired_l2a_pending_remains_visible_but_cannot_be_resumed` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:806-869`; `retired_r5_2_pending_is_rejected_with_retirement_cause_and_preserved` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:872-929`; `gpu_pending_is_bound_to_the_qualified_physical_identity` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:932-990`; `parakeet_pending_queue_resumes_only_the_same_model_source_and_prefix` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:993-1102`; `archive_admission_accepts_exactly_qualified_limit_and_refuses_one_extra_sample` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1105-1148`; `resume_refuses_over_limit_pending_and_preserves_its_bytes` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1151-1207`; `write_silence_wav` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1209-1231`; `pcm16_mono_wav` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1233-1267`; `r5_2_b9_reports_noise_floor_silences_and_cut_plans` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1271-1301`; `r5_2_a4_product_route_publishes_french_and_english_txt_srt` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1305-1434`; `decode_sha256` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1436-1442`; `path_var` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1493-1495`; `sha256` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1497-1500`; `runtime_paths` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1502-1522`; `runtime_hash` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1524-1532`; `start_process` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1534-1558`; `wait_event` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1560-1573`; `transcribe` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1575-1618`; `stop_process` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1620-1637`; `read_pcm16_mono` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1639-1674`; `write_pcm16_mono` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1676-1695`; `infer_reference` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1697-1710`; `infer_vad` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1712-1789`; `import_through_product_route` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1797-1897`; `verify_product_matches_vad_route` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1899-1941`; `decode_sha256` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1943-1949`; `normalized_tokens` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1951-1985`; `edit_distance` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:1987-2001`; `boundary_words` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:2003-2012`; `verify_comparison` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:2014-2067`; `en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:2071-2079`; `fr_synthetic_long_reference_and_vad_use_same_candidate` [tests] `crates/whisper-adapters/tests/parakeet_archive.rs:2083-2095`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_archive.rs:1-2096`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_archive.rs:1-2096`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `r5_2_long_tests`

Tests/auxiliaires : `path_var` `crates/whisper-adapters/tests/parakeet_archive.rs:1493-1495`, `sha256` `crates/whisper-adapters/tests/parakeet_archive.rs:1497-1500`, `runtime_paths` `crates/whisper-adapters/tests/parakeet_archive.rs:1502-1522`, `runtime_hash` `crates/whisper-adapters/tests/parakeet_archive.rs:1524-1532`. `crates/whisper-adapters/tests/parakeet_archive.rs:1444-2096`

Mesure locale : 0 production / 653 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/tests/parakeet_archive.rs:1444-2096`

Types locaux : `TimedWord` `crates/whisper-adapters/tests/parakeet_archive.rs:1480-1484`, `Inference` `crates/whisper-adapters/tests/parakeet_archive.rs:1486-1491`, `ProductImport` `crates/whisper-adapters/tests/parakeet_archive.rs:1791-1795`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/tests/parakeet_archive.rs:1444-2096`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### integration::parakeet_archive_personal_history — `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs`

Tests ou supports : définit `personal_archives_recognize_previous_workers_without_admitting_them_for_execution` `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:55-198`.

Mesure : 0 production / 198 tests / 198 physiques. `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:1-198`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:1-198`.

Fonctions avec corps : `key` [tests] `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:10-26`; `pending` [tests] `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:28-52`; `personal_archives_recognize_previous_workers_without_admitting_them_for_execution` [tests] `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:55-198`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:1-198`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_archive_personal_history.rs:1-198`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_command_wire — `crates/whisper-adapters/tests/parakeet_command_wire.rs`

Tests ou supports : définit `transcribe_v1_accepts_legacy_and_explicit_gpu_fields` `crates/whisper-adapters/tests/parakeet_command_wire.rs:46-84`, `shutdown_v1_has_the_existing_wire_shape` `crates/whisper-adapters/tests/parakeet_command_wire.rs:87-92`.

Mesure : 0 production / 92 tests / 92 physiques. `crates/whisper-adapters/tests/parakeet_command_wire.rs:1-92`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/parakeet_command_wire.rs:1-92`.

Fonctions avec corps : `expected_identity` [tests] `crates/whisper-adapters/tests/parakeet_command_wire.rs:23-30`; `expected_transcribe` [tests] `crates/whisper-adapters/tests/parakeet_command_wire.rs:32-43`; `transcribe_v1_accepts_legacy_and_explicit_gpu_fields` [tests] `crates/whisper-adapters/tests/parakeet_command_wire.rs:46-84`; `shutdown_v1_has_the_existing_wire_shape` [tests] `crates/whisper-adapters/tests/parakeet_command_wire.rs:87-92`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_command_wire.rs:1-92`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_command_wire.rs:1-92`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_control — `crates/whisper-adapters/tests/parakeet_control.rs`

Tests ou supports : définit `stop_kills_and_waits_for_the_isolated_worker_process` `crates/whisper-adapters/tests/parakeet_control.rs:21-49`, `stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication` `crates/whisper-adapters/tests/parakeet_control.rs:53-161`.

Mesure : 0 production / 940 tests / 940 physiques. `crates/whisper-adapters/tests/parakeet_control.rs:1-940`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/parakeet_control.rs:1-940`.

Fonctions avec corps : `stop_kills_and_waits_for_the_isolated_worker_process` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:21-49`; `stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:53-161`; `stop_and_quit_at_preparation_and_publication_barriers_preserve_truthful_state` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:165-227`; `stop_queue_journal_failure_settles_runtime_and_allows_next_import` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:231-404`; `resumed_native_import_keeps_confirmed_prefix_once_and_publishes_once` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:408-562`; `resumed_native_vad_import_keeps_confirmed_prefix_once_and_publishes_once` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:566-763`; `read_segment_rows` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:765-771`; `run_barrier_case` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:773-886`; `wait_for_state` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:888-906`; `passage_dir` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:908-912`; `required_path` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:914-916`; `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:918-932`; `decode_sha256` [tests] `crates/whisper-adapters/tests/parakeet_control.rs:934-940`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_control.rs:1-940`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_control.rs:1-940`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_import — `crates/whisper-adapters/tests/parakeet_import.rs`

Tests ou supports : définit `parakeet_worker_configuration_is_confined_to_the_l2a2_runtime` `crates/whisper-adapters/tests/parakeet_import.rs:24-123`, `gpu_app_fr_en_single_pass` `crates/whisper-adapters/tests/parakeet_import.rs:126-143`.

Mesure : 0 production / 3241 tests / 3241 physiques. `crates/whisper-adapters/tests/parakeet_import.rs:1-3241`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/parakeet_import.rs:1-3241`.

Fonctions avec corps : `parakeet_worker_configuration_is_confined_to_the_l2a2_runtime` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:24-123`; `gpu_app_fr_en_single_pass` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:126-143`; `l6_personal_application_recipe` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:148-189`; `l6_personal_gpu_manual_auto_routes` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:194-224`; `l6_personal_en5_twice_release` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:229-329`; `l6_personal_rejects_over_limit_before_pending` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:334-344`; `run_personal_application_import` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:347-365`; `run_personal_application_route` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:368-604`; `extend_pcm16_wav_by_one_sample` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:607-635`; `gpu_app_en5` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:638-649`; `gpu_app_en5_timing_diagnostic` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:652-667`; `gpu_app_en30` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:671-682`; `gpu_manual_and_auto_fallback` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:686-733`; `gpu_rejects_bad_identity_before_pending` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:737-754`; `recipe_selector_matches` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:756-767`; `selected_recipe` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:769-786`; `gpu_recipe_selector_preserves_l3b_l3c_and_supports_explicit_l3d` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:789-821`; `gpu_app_en5_twice_release` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:824-1086`; `run_gpu_application_import` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1088-1096`; `run_gpu_application_import_with_native` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1098-1331`; `gpu_identity_rejects_same_pci_with_a_different_windows_device` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1334-1340`; `adapter_diagnostics` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1342-1353`; `worker_stderr_since` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1355-1380`; `personal_worker_build` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1384-1400`; `qualified_gpu_candidate` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1402-1412`; `r6_application_single_pass_recipe` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1416-1550`; `r7_application_timing` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1554-1699`; `r9_application_recipe` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:1703-2108`; `pcm16_mono_sample_count` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2110-2153`; `incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2157-2383`; `ensure_staged_incompatible_native_fixtures` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2385-2463`; `copy_fixture_directory` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2465-2476`; `existing_incompatible_gpu_fixture` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2478-2500`; `assert_cpu_incompatible_native_refusal` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2502-2635`; `existing_cpu_incompatible_fixture` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2637-2684`; `killed_native_worker_is_reported_as_exit_without_result` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2688-2706`; `assert_worker_death` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2708-2724`; `assert_worker_death_with_gpu` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2726-2766`; `incomplete_result_frame_is_rejected_as_transport_decode_failure` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2770-2819`; `assert_incomplete_result_frame` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2821-2826`; `product_route_worker_death_preserves_pending_source_and_rejects_completion` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2830-2845`; `product_route_incomplete_result_preserves_pending_source_and_rejects_completion` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2849-2864`; `run_product_transport_failure` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:2866-3006`; `configured_paths` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3008-3015`; `configured_r6_cpu_paths` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3017-3024`; `configured_gpu_paths` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3026-3033`; `assert_registered_gpu_key` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3035-3075`; `gpu_worker_path` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3077-3085`; `gpu_native_dir` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3087-3095`; `wait_for_ready` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3097-3108`; `poll_until_error` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3110-3124`; `required_path` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3126-3128`; `not_run_without_campaign_variables` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3130-3144`; `cpu_worker_path` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3146-3151`; `cpu_native_dir` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3153-3158`; `r8_exec_root` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3160-3220`; `sha256` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3222-3227`; `decode_sha256` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3229-3235`; `passage_dir` [tests] `crates/whisper-adapters/tests/parakeet_import.rs:3237-3241`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_import.rs:1-3241`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_import.rs:1-3241`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_long — `crates/whisper-adapters/tests/parakeet_long.rs`

Tests ou supports : définit `en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy` `crates/whisper-adapters/tests/parakeet_long.rs:612-620`, `fr_synthetic_long_reference_and_vad_use_same_candidate` `crates/whisper-adapters/tests/parakeet_long.rs:624-636`.

Mesure : 0 production / 669 tests / 669 physiques. `crates/whisper-adapters/tests/parakeet_long.rs:1-669`

Types : `TimedWord` (struct, tests) `crates/whisper-adapters/tests/parakeet_long.rs:34-38`; `Inference` (struct, tests) `crates/whisper-adapters/tests/parakeet_long.rs:40-45`; `ProductImport` (struct, tests) `crates/whisper-adapters/tests/parakeet_long.rs:340-344`.

Fonctions avec corps : `path_var` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:47-49`; `sha256` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:51-54`; `runtime_paths` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:56-75`; `runtime_hash` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:77-85`; `start_process` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:87-108`; `wait_event` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:110-123`; `transcribe` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:125-168`; `stop_process` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:170-187`; `read_pcm16_mono` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:189-223`; `write_pcm16_mono` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:225-244`; `infer_reference` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:246-259`; `infer_vad` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:261-338`; `import_through_product_route` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:346-445`; `verify_product_matches_vad_route` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:447-485`; `decode_sha256` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:487-493`; `normalized_tokens` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:495-526`; `edit_distance` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:528-542`; `boundary_words` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:544-553`; `verify_comparison` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:555-608`; `en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:612-620`; `fr_synthetic_long_reference_and_vad_use_same_candidate` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:624-636`; `b9_reports_noise_floor_silence_counts_and_cut_plans` [tests] `crates/whisper-adapters/tests/parakeet_long.rs:640-669`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_long.rs:1-669`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_long.rs:1-669`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::parakeet_runtime_isolation — `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs`

Tests ou supports : définit `personal_import_is_independent_of_historical_environment` `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:12-14`, `historical_gpu_rejects_invalid_log_root_and_qualification` `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:18-85`.

Mesure : 0 production / 85 tests / 85 physiques. `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-85`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-85`.

Fonctions avec corps : `personal_import_is_independent_of_historical_environment` [tests] `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:12-14`; `historical_gpu_rejects_invalid_log_root_and_qualification` [tests] `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:18-85`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_adapters::parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};` `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:2`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-85`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `parakeet_import`

Tests/auxiliaires : déclarations de `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-3`. `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-3`

Mesure locale : 0 production / 3 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-3`

Types locaux : aucun relevé dans `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-3`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:1-3`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

#### Inline `support`

Tests/auxiliaires : `personal_import_is_independent_of_historical_environment` `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:12-14`. `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:5-15`

Mesure locale : 0 production / 11 tests ; sous-ensemble non additionnable. `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:5-15`

Types locaux : aucun relevé dans `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:5-15`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs:5-15`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### integration::parakeet_unified_native — `crates/whisper-adapters/tests/parakeet_unified_native.rs`

Tests ou supports : définit `build_french_and_english_long_session_fixtures_from_voted_sps_clips` `crates/whisper-adapters/tests/parakeet_unified_native.rs:738-754`, `build_physical_rotation_fixture_over_one_gibibyte` `crates/whisper-adapters/tests/parakeet_unified_native.rs:819-821`.

Mesure : 0 production / 933 tests / 933 physiques. `crates/whisper-adapters/tests/parakeet_unified_native.rs:1-933`

Types : `LongCorpusAnnotation` (struct, tests) `crates/whisper-adapters/tests/parakeet_unified_native.rs:402-410`; `LongCorpusSegment` (struct, tests) `crates/whisper-adapters/tests/parakeet_unified_native.rs:414-420`.

Fonctions avec corps : `required_path` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:20-27`; `fixture` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:29-36`; `sha256_file` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:38-52`; `personal_paths` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:54-112`; `request` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:114-145`; `run_native_import` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:147-398`; `run_native_long_case` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:423-581`; `build_long_fixture` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:584-733`; `build_french_and_english_long_session_fixtures_from_voted_sps_clips` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:738-754`; `build_physical_rotation_fixture` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:757-814`; `build_physical_rotation_fixture_over_one_gibibyte` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:819-821`; `personal_gpu_candidate` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:824-849`; `native_personal_cpu_imports_french_mp3_through_the_unified_application` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:854-863`; `native_personal_gpu_imports_english_mp3_with_attested_candidate` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:868-877`; `native_personal_auto_without_candidate_reports_preaudio_cpu_fallback` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:882-891`; `native_personal_cpu_imports_annotated_french_long_audio` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:896-898`; `native_personal_cpu_imports_annotated_english_long_audio` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:903-905`; `native_personal_gpu_imports_annotated_french_long_audio` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:910-912`; `native_personal_gpu_imports_annotated_english_long_audio` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:917-919`; `native_personal_auto_imports_annotated_french_long_audio` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:924-926`; `native_personal_auto_imports_annotated_english_long_audio` [tests] `crates/whisper-adapters/tests/parakeet_unified_native.rs:931-933`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/parakeet_unified_native.rs:1-933`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/parakeet_unified_native.rs:1-933`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::publication_lock_contract — `crates/whisper-adapters/tests/publication_lock_contract.rs`

Tests ou supports : définit `publication_lock_preserves_contents_and_exclusion_until_drop` `crates/whisper-adapters/tests/publication_lock_contract.rs:9-37`.

Mesure : 0 production / 37 tests / 37 physiques. `crates/whisper-adapters/tests/publication_lock_contract.rs:1-37`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/publication_lock_contract.rs:1-37`.

Fonctions avec corps : `publication_lock_preserves_contents_and_exclusion_until_drop` [tests] `crates/whisper-adapters/tests/publication_lock_contract.rs:9-37`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/publication_lock_contract.rs:1-37`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/publication_lock_contract.rs:1-37`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::reprocessing_baseline — `crates/whisper-adapters/tests/reprocessing_baseline.rs`

Tests ou supports : définit `r11_application_baseline` `crates/whisper-adapters/tests/reprocessing_baseline.rs:26-191`.

Mesure : 0 production / 548 tests / 548 physiques. `crates/whisper-adapters/tests/reprocessing_baseline.rs:1-548`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/reprocessing_baseline.rs:1-548`.

Fonctions avec corps : `r11_application_baseline` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:26-191`; `request` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:193-215`; `run_import` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:217-236`; `run_import_with_duplicate_choice` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:238-258`; `run_import_with_choice` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:260-334`; `wait_for_queued_request` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:336-357`; `operation_deadline` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:359-361`; `wait_for_complete` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:363-388`; `assert_outputs_for_source` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:390-406`; `assert_outputs_at` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:408-419`; `generation_dirs` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:421-438`; `required_path` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:440-444`; `validate_campaign_root` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:446-460`; `validate_destination_under_root` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:462-498`; `assert_inside` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:500-508`; `next_job_id` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:510-514`; `unix_nanos` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:516-521`; `sha256` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:523-526`; `decode_sha256` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:528-535`; `record_harness_timing` [tests] `crates/whisper-adapters/tests/reprocessing_baseline.rs:537-548`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/reprocessing_baseline.rs:1-548`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/reprocessing_baseline.rs:1-548`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::reprocessing_cleanup — `crates/whisper-adapters/tests/reprocessing_cleanup.rs`

Tests ou supports : définit `reparse_container_is_rejected_before_any_old_artifact_is_removed` `crates/whisper-adapters/tests/reprocessing_cleanup.rs:504-528`, `replaced_container_identity_is_rejected_and_both_directories_are_preserved` `crates/whisper-adapters/tests/reprocessing_cleanup.rs:531-552`.

Mesure : 0 production / 1598 tests / 1598 physiques. `crates/whisper-adapters/tests/reprocessing_cleanup.rs:1-1598`

Types : `IdentityBytes` (struct, tests) `crates/whisper-adapters/tests/reprocessing_cleanup.rs:81`; `ContainerMutation` (enum, tests) `crates/whisper-adapters/tests/reprocessing_cleanup.rs:312-315`; `FailedCleanupFixture` (struct, tests) `crates/whisper-adapters/tests/reprocessing_cleanup.rs:317-325`.

Fonctions avec corps : `root` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:21-33`; `wav` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:35-52`; `request` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:54-70`; `sha_hex` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:72-74`; `snapshot_registry` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:76-78`; `finish` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:84-86`; `write` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:87-89`; `persisted_identity` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:92-97`; `add_owned_and_unowned_export_fixtures` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:99-150`; `seed_treatment` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:154-156`; `seed_treatment_generation` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:158-230`; `historical_catalog_fixture` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:232-268`; `compile_worker` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:270-310`; `create_junction` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:327-344`; `create_failed_cleanup_fixture` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:346-501`; `reparse_container_is_rejected_before_any_old_artifact_is_removed` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:504-528`; `replaced_container_identity_is_rejected_and_both_directories_are_preserved` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:531-552`; `durable_legacy_cleanup_replay_keeps_empty_parent_without_container_proof` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:555-616`; `characterization_restore_cleanup_fixture` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:618-626`; `archive_characterization_cleanup_missing_object_before_progress_is_adopted` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:629-673`; `archive_characterization_cleanup_recorded_removed_but_present_is_preserved` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:676-722`; `replacement_reservation_ignores_historical_rows_without_treatments` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:725-799`; `replacement_reservation_still_rejects_a_changed_verified_catalog` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:802-842`; `replace_all_publishes_first_stops_partial_cleanup_and_resumes_only_on_request` [tests] `crates/whisper-adapters/tests/reprocessing_cleanup.rs:845-1598`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/reprocessing_cleanup.rs:1-1598`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/reprocessing_cleanup.rs:1-1598`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::reprocessing_create — `crates/whisper-adapters/tests/reprocessing_create.rs`

Tests ou supports : définit `create_new_publishes_only_its_internal_pair_and_preserves_existing_treatment` `crates/whisper-adapters/tests/reprocessing_create.rs:201-236`, `application_create_new_processes_the_bound_snapshot_and_preserves_prior_publication` `crates/whisper-adapters/tests/reprocessing_create.rs:239-403`.

Mesure : 0 production / 403 tests / 403 physiques. `crates/whisper-adapters/tests/reprocessing_create.rs:1-403`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/reprocessing_create.rs:1-403`.

Fonctions avec corps : `root` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:22-34`; `wav` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:36-53`; `request` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:55-71`; `sha_hex` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:73-75`; `registry` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:77-79`; `transcribe` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:81-152`; `compile_snapshot_worker` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:154-198`; `create_new_publishes_only_its_internal_pair_and_preserves_existing_treatment` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:201-236`; `application_create_new_processes_the_bound_snapshot_and_preserves_prior_publication` [tests] `crates/whisper-adapters/tests/reprocessing_create.rs:239-403`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/reprocessing_create.rs:1-403`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/reprocessing_create.rs:1-403`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::source_binding — `crates/whisper-adapters/tests/source_binding.rs`

Tests ou supports : définit `captures_once_reuses_snapshot_and_retire_releases_capacity` `crates/whisper-adapters/tests/source_binding.rs:69-125`, `rebinding_operation_moves_only_its_owned_snapshot_and_record` `crates/whisper-adapters/tests/source_binding.rs:128-142`.

Mesure : 0 production / 637 tests / 637 physiques. `crates/whisper-adapters/tests/source_binding.rs:1-637`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/source_binding.rs:1-637`.

Fonctions avec corps : `root` [tests] `crates/whisper-adapters/tests/source_binding.rs:12-26`; `wav` [tests] `crates/whisper-adapters/tests/source_binding.rs:28-44`; `request` [tests] `crates/whisper-adapters/tests/source_binding.rs:46-62`; `registry` [tests] `crates/whisper-adapters/tests/source_binding.rs:64-66`; `captures_once_reuses_snapshot_and_retire_releases_capacity` [tests] `crates/whisper-adapters/tests/source_binding.rs:69-125`; `rebinding_operation_moves_only_its_owned_snapshot_and_record` [tests] `crates/whisper-adapters/tests/source_binding.rs:128-142`; `conflicting_bound_snapshot_path_is_refused_and_keeps_both_slots` [tests] `crates/whisper-adapters/tests/source_binding.rs:145-197`; `rebind_rejects_operation_identity_owned_in_another_destination` [tests] `crates/whisper-adapters/tests/source_binding.rs:200-239`; `explicit_recovery_releases_only_verified_abandoned_slots_when_destination_is_empty` [tests] `crates/whisper-adapters/tests/source_binding.rs:242-288`; `explicit_recovery_preserves_active_and_incomplete_bindings` [tests] `crates/whisper-adapters/tests/source_binding.rs:291-356`; `explicit_recovery_keeps_a_snapshot_referenced_by_the_durable_queue` [tests] `crates/whisper-adapters/tests/source_binding.rs:359-391`; `explicit_recovery_keeps_a_snapshot_with_a_publication_reservation` [tests] `crates/whisper-adapters/tests/source_binding.rs:394-428`; `explicit_recovery_preserves_binding_when_another_generation_has_corrupt_pending` [tests] `crates/whisper-adapters/tests/source_binding.rs:431-481`; `durable_global_capacity_spans_destinations_and_restarts` [tests] `crates/whisper-adapters/tests/source_binding.rs:484-525`; `same_size_snapshot_mutation_is_rejected_and_keeps_its_global_slot` [tests] `crates/whisper-adapters/tests/source_binding.rs:528-572`; `interrupted_empty_reservation_is_not_reused_after_restart` [tests] `crates/whisper-adapters/tests/source_binding.rs:575-616`; `malformed_global_registry_fails_closed` [tests] `crates/whisper-adapters/tests/source_binding.rs:619-637`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/source_binding.rs:1-637`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/source_binding.rs:1-637`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::support::parakeet_fake_worker — `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs`

Tests ou supports : définit `main` `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:3-31`.

Mesure : 0 production / 31 tests / 31 physiques. `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:1-31`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:1-31`.

Fonctions avec corps : `main` [tests] `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:3-31`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:1-31`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/support/parakeet_fake_worker.rs:1-31`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::parakeet_import::runtime_isolation_tests / integration::parakeet_runtime_isolation / integration::support::parakeet_runtime_isolation — `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs`

Tests ou supports : définit `run_matrix` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:20-197`.

Mesure : 0 production / 197 tests / 197 physiques. `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1-197`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1-197`.

Fonctions avec corps : `run_matrix` [tests] `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:20-197`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1-197`.

Dépend de, via `use crate::` : tests : `crate::parakeet_import::ParakeetProcess` → `crate::parakeet_import` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1`<br>tests : `crate::parakeet_import::ParakeetWorkerPaths` → `crate::parakeet_import` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1`<br>tests : `crate::parakeet_import` → `crate::parakeet_import` `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::vad_contract — `crates/whisper-adapters/tests/vad_contract.rs`

Tests ou supports : définit `vad_accepts_only_20ms_mono_16khz_frames` `crates/whisper-adapters/tests/vad_contract.rs:4-8`.

Mesure : 0 production / 8 tests / 8 physiques. `crates/whisper-adapters/tests/vad_contract.rs:1-8`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/vad_contract.rs:1-8`.

Fonctions avec corps : `vad_accepts_only_20ms_mono_16khz_frames` [tests] `crates/whisper-adapters/tests/vad_contract.rs:4-8`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/vad_contract.rs:1-8`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/vad_contract.rs:1-8`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::worker_control — `crates/whisper-adapters/tests/worker_control.rs`

Tests ou supports : définit `cpu_choice_uses_only_cpu_worker_even_when_gpu_exists` `crates/whisper-adapters/tests/worker_control.rs:28-36`, `auto_prefers_gpu_and_uses_cpu_when_gpu_binary_is_absent` `crates/whisper-adapters/tests/worker_control.rs:39-50`.

Mesure : 0 production / 102 tests / 102 physiques. `crates/whisper-adapters/tests/worker_control.rs:1-102`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-adapters/tests/worker_control.rs:1-102`.

Fonctions avec corps : `paths` [tests] `crates/whisper-adapters/tests/worker_control.rs:13-25`; `cpu_choice_uses_only_cpu_worker_even_when_gpu_exists` [tests] `crates/whisper-adapters/tests/worker_control.rs:28-36`; `auto_prefers_gpu_and_uses_cpu_when_gpu_binary_is_absent` [tests] `crates/whisper-adapters/tests/worker_control.rs:39-50`; `forced_gpu_fails_closed_when_worker_is_absent` [tests] `crates/whisper-adapters/tests/worker_control.rs:53-61`; `missing_progress_warns_once_without_turning_into_a_stop` [tests] `crates/whisper-adapters/tests/worker_control.rs:64-79`; `capture_progress_does_not_hide_a_stalled_inference_obligation` [tests] `crates/whisper-adapters/tests/worker_control.rs:82-102`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-adapters/tests/worker_control.rs:1-102`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-adapters/tests/worker_control.rs:1-102`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 9 | 1 |
| cfg_feature | 74 | 30 |
| cfg_test | 0 | 266 |
| command_new | 3 | 26 |
| expect | 45 | 548 |
| thread_spawn | 7 | 21 |
| unsafe | 12 | 0 |
| unwrap | 13 | 4370 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 317 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 128 sur plages hors tests et 189 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::chunks_exact_to_as_chunks | 0 | 12 |
| clippy::cloned_ref_to_slice_refs | 0 | 2 |
| clippy::cognitive_complexity | 10 | 35 |
| clippy::collapsible_if | 0 | 9 |
| clippy::drop_non_drop | 0 | 2 |
| clippy::if_same_then_else | 0 | 2 |
| clippy::items_after_test_module | 0 | 2 |
| clippy::len_zero | 0 | 1 |
| clippy::let_and_return | 8 | 0 |
| clippy::missing_const_for_thread_local | 0 | 1 |
| clippy::needless_borrow | 0 | 1 |
| clippy::needless_borrows_for_generic_args | 0 | 1 |
| clippy::needless_return | 0 | 1 |
| clippy::question_mark | 0 | 1 |
| clippy::single_element_loop | 0 | 1 |
| clippy::too_many_arguments | 0 | 3 |
| clippy::too_many_lines | 108 | 92 |
| clippy::type_complexity | 0 | 7 |
| dead_code | 0 | 3 |
| unused_assignments | 0 | 6 |
| unused_imports | 2 | 2 |
| unused_mut | 0 | 4 |
| unused_variables | 0 | 1 |
