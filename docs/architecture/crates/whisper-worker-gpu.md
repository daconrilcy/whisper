Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-worker-gpu

## Rôle et type

Exécute import, live, décodage et qualification GPU depuis le protocole IPC. `crates/whisper-worker-gpu/src/session/mod.rs:1-335` ; `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100` ; `crates/whisper-worker-gpu/src/native_engine.rs:1-120` ; `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`

Charge CUDA et rapproche le périphérique natif de l’identité demandée. `crates/whisper-worker-gpu/src/session/mod.rs:1-335` ; `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100` ; `crates/whisper-worker-gpu/src/native_engine.rs:1-120` ; `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`

Type : **Worker binaire CUDA**. `crates/whisper-worker-gpu/src/session/mod.rs:1-335` ; `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100` ; `crates/whisper-worker-gpu/src/native_engine.rs:1-120` ; `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`

Mesures : **2843 production / 1320 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-worker-common | interne | dependencies | {"workspace": true} | `crates/whisper-worker-gpu/Cargo.toml:15` |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-worker-gpu/Cargo.toml:16` |
| serde_json | externe | dependencies | {"workspace": true} | `crates/whisper-worker-gpu/Cargo.toml:17` |
| serde | externe | dependencies | {"version": "=1.0.228", "features": ["derive"]} | `crates/whisper-worker-gpu/Cargo.toml:18` |
| sha2 | externe | dependencies | "=0.10.9" | `crates/whisper-worker-gpu/Cargo.toml:19` |
| whisper-rs | externe | dependencies | {"version": "=0.16.0", "default-features": false, "features": ["cuda", "raw-api"]} | `crates/whisper-worker-gpu/Cargo.toml:20` |
| windows | externe | cfg(target_os = "windows") / dependencies | {"version": "=0.62.2", "features": ["Win32_Foundation", "Win32_System_LibraryLoader", "Win32_Graphics_Dxgi", "Win32_Graphics_Dxgi_Common", "Win32_Devices_Display", "Win32_Devices_DeviceAndDriverInstallation"]} | `crates/whisper-worker-gpu/Cargo.toml:23` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-worker-gpu/Cargo.toml:1-29`.

## Interfaces publiques et usages intercrates

[Non vérifié] Aucun import consommateur d’une autre crate relevé ; les autres usages ne sont pas exclus.

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-worker-gpu/src/main.rs:1-20`. Imports textuels, pas un graphe de symboles résolu.

Interface entre processus : `crates/whisper-core/src/ipc.rs:1-343`.

Cible de build partagée : `scripts/recette-build.rs:1-7` (7 lignes hors tests, comptées une seule fois hors des totaux par crate). Elle refuse recette-hooks en release. Déclaration : `crates/whisper-worker-gpu/Cargo.toml:7`.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | `crates/whisper-worker-gpu/src/memory_qualification.rs:3` ; `crates/whisper-worker-gpu/src/qualification.rs:31` ; `crates/whisper-worker-gpu/src/session/import.rs:64` |
| Processus | `crates/whisper-worker-gpu/src/memory_qualification.rs:71` |
| Threads/canaux | `crates/whisper-worker-gpu/src/qualification.rs:294` ; `crates/whisper-worker-gpu/src/session/control.rs:37` ; `crates/whisper-worker-gpu/src/session/mod.rs:18` ; `crates/whisper-worker-gpu/src/startup.rs:9` ; `crates/whisper-worker-gpu/src/worker.rs:2` |
| Entrées/sorties standard | `crates/whisper-worker-gpu/src/memory_qualification.rs:73` ; `crates/whisper-worker-gpu/src/startup.rs:17` |
| GPU/natif | `crates/whisper-worker-gpu/src/gpu_commands.rs:65` ; `crates/whisper-worker-gpu/src/gpu_qualification.rs:6` ; `crates/whisper-worker-gpu/src/native_engine.rs:1` ; `crates/whisper-worker-gpu/src/native_ffi.rs:315` ; `crates/whisper-worker-gpu/src/session/mod.rs:105` |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate::decoder::tests | Tests ou supports : définit `inaccessible_source_propagates_error` `crates/whisper-worker-gpu/src/decoder/tests.rs:4-11`. | 0 | 11 |
| crate::decoder | Adapte les fonctions de décodage partagées à l’observateur local. `crates/whisper-worker-gpu/src/decoder.rs:1-29` | 27 | 2 |
| crate::diagnostics::tests | Tests ou supports : définit `verified_model_gpu_initialization_failure_is_retryable_and_correlated` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:6-51`, `gpu_revalidation_failure_is_typed_and_bound_to_the_startup_attempt` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:54-76`. | 0 | 76 |
| crate::diagnostics | Émet les événements IPC de défaillance. `crates/whisper-worker-gpu/src/diagnostics.rs:1-61` | 59 | 2 |
| crate::encoder | Réexporte l’encodeur MP3 partagé. `crates/whisper-worker-gpu/src/encoder.rs:1` | 1 | 0 |
| crate::engine_cache::tests | Tests ou supports : définit `cache_target_matches_complete_device_identity` `crates/whisper-worker-gpu/src/engine_cache/tests.rs:17-27`, `cached_engine_is_attested_again_and_evicted_on_failure` `crates/whisper-worker-gpu/src/engine_cache/tests.rs:30-44`. | 0 | 44 |
| crate::engine_cache | Vérifie l’identité demandée avant réutilisation ou chargement du moteur. `crates/whisper-worker-gpu/src/engine_cache.rs:1-83` | 81 | 2 |
| crate::gpu_attestation | Compare l’identité GPU native au candidat demandé. `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90` | 49 | 41 |
| crate::gpu_commands | Traite les commandes de probe et qualification GPU. `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100` | 100 | 0 |
| crate::gpu_qualification | Qualifie les périphériques demandés et leurs identifiants. `crates/whisper-worker-gpu/src/gpu_qualification.rs:1-293` | 133 | 160 |
| crate::integrity | Réexporte la vérification SHA-256 partagée. `crates/whisper-worker-gpu/src/integrity.rs:1` | 1 | 0 |
| crate::ipc::tests | Tests ou supports : définit `rejects_oversized_and_incomplete_frames_before_deserialization` `crates/whisper-worker-gpu/src/ipc/tests.rs:4-20`, `writer_accepts_limit_and_rejects_limit_plus_one` `crates/whisper-worker-gpu/src/ipc/tests.rs:23-34`. | 0 | 34 |
| crate::ipc | Adapte les trames partagées à l’observateur local et définit Incoming. `crates/whisper-worker-gpu/src/ipc.rs:1-30` | 26 | 4 |
| crate | Délègue le point d’entrée à startup::run. `crates/whisper-worker-gpu/src/main.rs:1-20` | 20 | 0 |
| crate::memory_qualification | Écrit les mesures mémoire conditionnelles du worker. `crates/whisper-worker-gpu/src/memory_qualification.rs:1-92` | 92 | 0 |
| crate::native_engine | Charge whisper-rs et transforme ses résultats de transcription. `crates/whisper-worker-gpu/src/native_engine.rs:1-120` | 120 | 0 |
| crate::native_ffi | Interroge CUDA/GGML et corrèle identités natives/Windows et traces GPU. `crates/whisper-worker-gpu/src/native_ffi.rs:1-623` | 557 | 66 |
| crate::qualification | Implémente résultats/barrières de campagne L04 conditionnés par recette-hooks. `crates/whisper-worker-gpu/src/qualification.rs:1-307` | 307 | 0 |
| crate::session::control::tests | Tests ou supports : définit `empty_and_disconnected_channels_do_not_stop_the_session` `crates/whisper-worker-gpu/src/session/control/tests.rs:4-11`, `wrong_stop_is_ignored_and_matching_stop_is_consumed` `crates/whisper-worker-gpu/src/session/control/tests.rs:14-27`. | 0 | 47 |
| crate::session::control | Draine les commandes d’arrêt et contrôle leur identité. `crates/whisper-worker-gpu/src/session/control.rs:1-44` | 42 | 2 |
| crate::session::decode | Traite DecodeBlock et émet les blocs PCM. `crates/whisper-worker-gpu/src/session/decode.rs:1-148` | 148 | 0 |
| crate::session::import | Vérifie source/modèle et exécute la transcription import par fenêtres. `crates/whisper-worker-gpu/src/session/import.rs:1-371` | 371 | 0 |
| crate::session::live | Exécute les commandes audio live et émet segments, paquets et fins de fenêtres. `crates/whisper-worker-gpu/src/session/live.rs:1-254` | 254 | 0 |
| crate::session | Vérifie le protocole et route les commandes import/live/décodage. `crates/whisper-worker-gpu/src/session/mod.rs:1-335` | 335 | 0 |
| crate::startup | Ouvre stdin/stdout, crée le canal et lance le thread lecteur IPC. `crates/whisper-worker-gpu/src/startup.rs:1-82` | 82 | 0 |
| crate::worker::tests | Tests ou supports : définit `idle_worker_shutdown_exits_without_an_error_frame` `crates/whisper-worker-gpu/src/worker/tests.rs:6-14`. | 0 | 14 |
| crate::worker | Boucle sur les sessions et conserve le cache moteur entre demandes. `crates/whisper-worker-gpu/src/worker.rs:1-40` | 38 | 2 |
| integration::gpu_selection_native | Tests ou supports : définit `linked_gpu_worker_answers_a_native_cuda_probe_without_loading_a_model` `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:8-31`. | 0 | 31 |
| integration::support::worker / integration::worker_contract::support | Tests ou supports : définit `deadline_child_fixture` `crates/whisper-worker-gpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-gpu/tests/support/worker.rs:226-259`. | 0 | 282 |
| integration::worker_contract | Tests ou supports : définit `turbo_idle_and_refusal_contract` `crates/whisper-worker-gpu/tests/worker_contract.rs:29-63`, `gpu_identity_contract` `crates/whisper-worker-gpu/tests/worker_contract.rs:66-103`. | 0 | 500 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| tests | externe | tests | crates/whisper-worker-gpu/src/decoder/tests.rs | `crates/whisper-worker-gpu/src/decoder.rs:28-29` |
| tests | externe | tests | crates/whisper-worker-gpu/src/diagnostics/tests.rs | `crates/whisper-worker-gpu/src/diagnostics.rs:60-61` |
| tests | externe | tests | crates/whisper-worker-gpu/src/engine_cache/tests.rs | `crates/whisper-worker-gpu/src/engine_cache.rs:3-4` |
| tests | inline | tests | plage locale | `crates/whisper-worker-gpu/src/gpu_attestation.rs:50-90` |
| tests | inline | tests | plage locale | `crates/whisper-worker-gpu/src/gpu_qualification.rs:134-293` |
| tests | externe | tests | crates/whisper-worker-gpu/src/ipc/tests.rs | `crates/whisper-worker-gpu/src/ipc.rs:29-30` |
| decoder | externe | hors tests / cfg | crates/whisper-worker-gpu/src/decoder.rs | `crates/whisper-worker-gpu/src/main.rs:1` |
| diagnostics | externe | hors tests / cfg | crates/whisper-worker-gpu/src/diagnostics.rs | `crates/whisper-worker-gpu/src/main.rs:2` |
| encoder | externe | hors tests / cfg | crates/whisper-worker-gpu/src/encoder.rs | `crates/whisper-worker-gpu/src/main.rs:3` |
| engine_cache | externe | hors tests / cfg | crates/whisper-worker-gpu/src/engine_cache.rs | `crates/whisper-worker-gpu/src/main.rs:4` |
| gpu_attestation | externe | hors tests / cfg | crates/whisper-worker-gpu/src/gpu_attestation.rs | `crates/whisper-worker-gpu/src/main.rs:5` |
| gpu_commands | externe | hors tests / cfg | crates/whisper-worker-gpu/src/gpu_commands.rs | `crates/whisper-worker-gpu/src/main.rs:6` |
| gpu_qualification | externe | hors tests / cfg | crates/whisper-worker-gpu/src/gpu_qualification.rs | `crates/whisper-worker-gpu/src/main.rs:7` |
| integrity | externe | hors tests / cfg | crates/whisper-worker-gpu/src/integrity.rs | `crates/whisper-worker-gpu/src/main.rs:8` |
| ipc | externe | hors tests / cfg | crates/whisper-worker-gpu/src/ipc.rs | `crates/whisper-worker-gpu/src/main.rs:9` |
| memory_qualification | externe | hors tests / cfg | crates/whisper-worker-gpu/src/memory_qualification.rs | `crates/whisper-worker-gpu/src/main.rs:10-11` |
| native_engine | externe | hors tests / cfg | crates/whisper-worker-gpu/src/native_engine.rs | `crates/whisper-worker-gpu/src/main.rs:12` |
| native_ffi | externe | hors tests / cfg | crates/whisper-worker-gpu/src/native_ffi.rs | `crates/whisper-worker-gpu/src/main.rs:13` |
| qualification | externe | hors tests / cfg | crates/whisper-worker-gpu/src/qualification.rs | `crates/whisper-worker-gpu/src/main.rs:14` |
| session | externe | hors tests / cfg | crates/whisper-worker-gpu/src/session/mod.rs | `crates/whisper-worker-gpu/src/main.rs:15` |
| startup | externe | hors tests / cfg | crates/whisper-worker-gpu/src/startup.rs | `crates/whisper-worker-gpu/src/main.rs:16` |
| worker | externe | hors tests / cfg | crates/whisper-worker-gpu/src/worker.rs | `crates/whisper-worker-gpu/src/main.rs:17` |
| tests | inline | tests | plage locale | `crates/whisper-worker-gpu/src/native_ffi.rs:166-231` |
| tests | externe | tests | crates/whisper-worker-gpu/src/session/control/tests.rs | `crates/whisper-worker-gpu/src/session/control.rs:43-44` |
| control | externe | hors tests / cfg | crates/whisper-worker-gpu/src/session/control.rs | `crates/whisper-worker-gpu/src/session/mod.rs:30` |
| decode | externe | hors tests / cfg | crates/whisper-worker-gpu/src/session/decode.rs | `crates/whisper-worker-gpu/src/session/mod.rs:31` |
| import | externe | hors tests / cfg | crates/whisper-worker-gpu/src/session/import.rs | `crates/whisper-worker-gpu/src/session/mod.rs:32` |
| live | externe | hors tests / cfg | crates/whisper-worker-gpu/src/session/live.rs | `crates/whisper-worker-gpu/src/session/mod.rs:33` |
| tests | externe | tests | crates/whisper-worker-gpu/src/worker/tests.rs | `crates/whisper-worker-gpu/src/worker.rs:39-40` |
| tests | inline | tests | plage locale | `crates/whisper-worker-gpu/tests/support/worker.rs:212-282` |
| support | externe | tests | crates/whisper-worker-gpu/tests/support/worker.rs | `crates/whisper-worker-gpu/tests/worker_contract.rs:15-16` |
| gpu_reuse | inline | tests | plage locale | `crates/whisper-worker-gpu/tests/worker_contract.rs:286-500` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate::decoder::tests — `crates/whisper-worker-gpu/src/decoder/tests.rs`

Tests ou supports : définit `inaccessible_source_propagates_error` `crates/whisper-worker-gpu/src/decoder/tests.rs:4-11`.

Mesure : 0 production / 11 tests / 11 physiques. `crates/whisper-worker-gpu/src/decoder/tests.rs:1-11`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/decoder/tests.rs:1-11`.

Fonctions avec corps : `inaccessible_source_propagates_error` [tests] `crates/whisper-worker-gpu/src/decoder/tests.rs:4-11`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/decoder/tests.rs:1-11`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/decoder/tests.rs:1-11`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::decoder — `crates/whisper-worker-gpu/src/decoder.rs`

Adapte les fonctions de décodage partagées à l’observateur local. `crates/whisper-worker-gpu/src/decoder.rs:1-29`

Mesure : 27 production / 2 tests / 29 physiques. `crates/whisper-worker-gpu/src/decoder.rs:1-29`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/decoder.rs:1-29`.

Fonctions avec corps : `observer` `crates/whisper-worker-gpu/src/decoder.rs:6-15`; `decode_requested_block` `crates/whisper-worker-gpu/src/decoder.rs:17-19`; `decode_wav_windows` `crates/whisper-worker-gpu/src/decoder.rs:21-26`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_worker_common::decoder::{MAX_WINDOW_SAMPLES, OUTPUT_RATE};` `crates/whisper-worker-gpu/src/decoder.rs:4`; `pub fn decode_requested_block(request: &DecodeRequest) -> Result<Option<Vec<i16>>, String> {` `crates/whisper-worker-gpu/src/decoder.rs:17`; `pub fn decode_wav_windows(` `crates/whisper-worker-gpu/src/decoder.rs:21`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/decoder.rs:1-29`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::diagnostics::tests — `crates/whisper-worker-gpu/src/diagnostics/tests.rs`

Tests ou supports : définit `verified_model_gpu_initialization_failure_is_retryable_and_correlated` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:6-51`, `gpu_revalidation_failure_is_typed_and_bound_to_the_startup_attempt` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:54-76`.

Mesure : 0 production / 76 tests / 76 physiques. `crates/whisper-worker-gpu/src/diagnostics/tests.rs:1-76`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/diagnostics/tests.rs:1-76`.

Fonctions avec corps : `verified_model_gpu_initialization_failure_is_retryable_and_correlated` [tests] `crates/whisper-worker-gpu/src/diagnostics/tests.rs:6-51`; `gpu_revalidation_failure_is_typed_and_bound_to_the_startup_attempt` [tests] `crates/whisper-worker-gpu/src/diagnostics/tests.rs:54-76`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/diagnostics/tests.rs:1-76`.

Dépend de, via `use crate::` : tests : `crate::integrity::verify_sha256` → `crate::integrity` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:2`<br>tests : `crate::ipc::read_frame` → `crate::ipc` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::diagnostics — `crates/whisper-worker-gpu/src/diagnostics.rs`

Émet les événements IPC de défaillance. `crates/whisper-worker-gpu/src/diagnostics.rs:1-61`

Mesure : 59 production / 2 tests / 61 physiques. `crates/whisper-worker-gpu/src/diagnostics.rs:1-61`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/diagnostics.rs:1-61`.

Fonctions avec corps : `emit_failure` `crates/whisper-worker-gpu/src/diagnostics.rs:5-24`; `emit_gpu_initialization_failure` `crates/whisper-worker-gpu/src/diagnostics.rs:26-41`; `emit_gpu_revalidation_failure` `crates/whisper-worker-gpu/src/diagnostics.rs:43-58`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/diagnostics.rs:1-61`.

Dépend de, via `use crate::` : hors tests : `crate::ipc::write_frame` → `crate::ipc` `crates/whisper-worker-gpu/src/diagnostics.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::encoder — `crates/whisper-worker-gpu/src/encoder.rs`

Réexporte l’encodeur MP3 partagé. `crates/whisper-worker-gpu/src/encoder.rs:1`

Mesure : 1 production / 0 tests / 1 physiques. `crates/whisper-worker-gpu/src/encoder.rs:1`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/encoder.rs:1`.

Fonctions avec corps : aucune relevée dans `crates/whisper-worker-gpu/src/encoder.rs:1`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_worker_common::encoder::LiveMp3Encoder;` `crates/whisper-worker-gpu/src/encoder.rs:1`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/encoder.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::engine_cache::tests — `crates/whisper-worker-gpu/src/engine_cache/tests.rs`

Tests ou supports : définit `cache_target_matches_complete_device_identity` `crates/whisper-worker-gpu/src/engine_cache/tests.rs:17-27`, `cached_engine_is_attested_again_and_evicted_on_failure` `crates/whisper-worker-gpu/src/engine_cache/tests.rs:30-44`.

Mesure : 0 production / 44 tests / 44 physiques. `crates/whisper-worker-gpu/src/engine_cache/tests.rs:1-44`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/engine_cache/tests.rs:1-44`.

Fonctions avec corps : `identity` [tests] `crates/whisper-worker-gpu/src/engine_cache/tests.rs:4-14`; `cache_target_matches_complete_device_identity` [tests] `crates/whisper-worker-gpu/src/engine_cache/tests.rs:17-27`; `cached_engine_is_attested_again_and_evicted_on_failure` [tests] `crates/whisper-worker-gpu/src/engine_cache/tests.rs:30-44`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/engine_cache/tests.rs:1-44`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/engine_cache/tests.rs:1-44`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::engine_cache — `crates/whisper-worker-gpu/src/engine_cache.rs`

Vérifie l’identité demandée avant réutilisation ou chargement du moteur. `crates/whisper-worker-gpu/src/engine_cache.rs:1-83`

Mesure : 81 production / 2 tests / 83 physiques. `crates/whisper-worker-gpu/src/engine_cache.rs:1-83`

Types : `CachedGpuEngine` (struct, hors tests) `crates/whisper-worker-gpu/src/engine_cache.rs:6-11`.

Fonctions avec corps : `same_gpu_target` `crates/whisper-worker-gpu/src/engine_cache.rs:13-31`; `attest_cached` `crates/whisper-worker-gpu/src/engine_cache.rs:33-42`; `cached_gpu_engine` `crates/whisper-worker-gpu/src/engine_cache.rs:44-83`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/engine_cache.rs:1-83`.

Dépend de, via `use crate::` : hors tests : `crate::native_engine::GpuEngine` → `crate::native_engine` `crates/whisper-worker-gpu/src/engine_cache.rs:1`<br>hors tests : `crate::native_ffi` → `crate::native_ffi` `crates/whisper-worker-gpu/src/engine_cache.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::worker` `crates/whisper-worker-gpu/src/worker.rs:1`.

### crate::gpu_attestation — `crates/whisper-worker-gpu/src/gpu_attestation.rs`

Compare l’identité GPU native au candidat demandé. `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`

Mesure : 49 production / 41 tests / 90 physiques. `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`.

Fonctions avec corps : `attest_native_identity` `crates/whisper-worker-gpu/src/gpu_attestation.rs:3-40`; `normalize_pci` `crates/whisper-worker-gpu/src/gpu_attestation.rs:42-48`; `candidate` [tests] `crates/whisper-worker-gpu/src/gpu_attestation.rs:54-64`; `cuda` [tests] `crates/whisper-worker-gpu/src/gpu_attestation.rs:66-73`; `ggml` [tests] `crates/whisper-worker-gpu/src/gpu_attestation.rs:75-81`; `reattestation_requires_exact_cuda_and_ggml_ordinals_and_identity` [tests] `crates/whisper-worker-gpu/src/gpu_attestation.rs:84-89`.

Déclarations `pub` (visibilité sous conditions) : `pub fn attest_native_identity(` `crates/whisper-worker-gpu/src/gpu_attestation.rs:3`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/gpu_attestation.rs:1-90`.

Utilisé par, via ces imports : hors tests : `crate::gpu_commands` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

#### Inline `tests`

Tests/auxiliaires : `candidate` `crates/whisper-worker-gpu/src/gpu_attestation.rs:54-64`, `cuda` `crates/whisper-worker-gpu/src/gpu_attestation.rs:66-73`, `ggml` `crates/whisper-worker-gpu/src/gpu_attestation.rs:75-81`, `reattestation_requires_exact_cuda_and_ggml_ordinals_and_identity` `crates/whisper-worker-gpu/src/gpu_attestation.rs:84-89`. `crates/whisper-worker-gpu/src/gpu_attestation.rs:50-90`

Mesure locale : 0 production / 41 tests ; sous-ensemble non additionnable. `crates/whisper-worker-gpu/src/gpu_attestation.rs:50-90`

Types locaux : aucun relevé dans `crates/whisper-worker-gpu/src/gpu_attestation.rs:50-90`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-gpu/src/gpu_attestation.rs:50-90`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::gpu_commands — `crates/whisper-worker-gpu/src/gpu_commands.rs`

Traite les commandes de probe et qualification GPU. `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100`

Mesure : 100 production / 0 tests / 100 physiques. `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100`.

Fonctions avec corps : `handle` `crates/whisper-worker-gpu/src/gpu_commands.rs:7-100`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/gpu_commands.rs:1-100`.

Dépend de, via `use crate::` : hors tests : `crate::gpu_attestation` → `crate::gpu_attestation` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::gpu_qualification` → `crate::gpu_qualification` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::integrity::verify_sha256` → `crate::integrity` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::ipc::write_frame` → `crate::ipc` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::native_ffi` → `crate::native_ffi` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::gpu_qualification — `crates/whisper-worker-gpu/src/gpu_qualification.rs`

Qualifie les périphériques demandés et leurs identifiants. `crates/whisper-worker-gpu/src/gpu_qualification.rs:1-293`

Mesure : 133 production / 160 tests / 293 physiques. `crates/whisper-worker-gpu/src/gpu_qualification.rs:1-293`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/gpu_qualification.rs:1-293`.

Fonctions avec corps : `qualify` `crates/whisper-worker-gpu/src/gpu_qualification.rs:5-15`; `qualify_one` `crates/whisper-worker-gpu/src/gpu_qualification.rs:17-124`; `normalize_pci` `crates/whisper-worker-gpu/src/gpu_qualification.rs:126-132`; `unique_luid_pci_ggml_join_requires_covered_architecture_and_ready_backend` [tests] `crates/whisper-worker-gpu/src/gpu_qualification.rs:140-187`; `duplicate_or_missing_join_endpoints_never_become_supported` [tests] `crates/whisper-worker-gpu/src/gpu_qualification.rs:190-231`; `missing_driver_or_required_symbol_is_unavailable_without_support_claim` [tests] `crates/whisper-worker-gpu/src/gpu_qualification.rs:234-254`; `intel_exclusion_uses_windows_identity_and_build_capability_without_cuda_join` [tests] `crates/whisper-worker-gpu/src/gpu_qualification.rs:257-292`.

Déclarations `pub` (visibilité sous conditions) : `pub const BUILD_ARCHITECTURES: &str = env!("CMAKE_CUDA_ARCHITECTURES");` `crates/whisper-worker-gpu/src/gpu_qualification.rs:3`; `pub fn qualify(targets: &[GpuQualificationTarget]) -> Vec<GpuQualificationResult> {` `crates/whisper-worker-gpu/src/gpu_qualification.rs:5`.

Dépend de, via `use crate::` : tests : `crate::native_ffi::CudaDevice` → `crate::native_ffi` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`<br>tests : `crate::native_ffi::GgmlDevice` → `crate::native_ffi` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`<br>tests : `crate::native_ffi` → `crate::native_ffi` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`.

Utilisé par, via ces imports : hors tests : `crate::gpu_commands` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`.

#### Inline `tests`

Tests/auxiliaires : `unique_luid_pci_ggml_join_requires_covered_architecture_and_ready_backend` `crates/whisper-worker-gpu/src/gpu_qualification.rs:140-187`, `duplicate_or_missing_join_endpoints_never_become_supported` `crates/whisper-worker-gpu/src/gpu_qualification.rs:190-231`, `missing_driver_or_required_symbol_is_unavailable_without_support_claim` `crates/whisper-worker-gpu/src/gpu_qualification.rs:234-254`, `intel_exclusion_uses_windows_identity_and_build_capability_without_cuda_join` `crates/whisper-worker-gpu/src/gpu_qualification.rs:257-292`. `crates/whisper-worker-gpu/src/gpu_qualification.rs:134-293`

Mesure locale : 0 production / 160 tests ; sous-ensemble non additionnable. `crates/whisper-worker-gpu/src/gpu_qualification.rs:134-293`

Types locaux : aucun relevé dans `crates/whisper-worker-gpu/src/gpu_qualification.rs:134-293`.

Imports locaux : `crate::native_ffi::CudaDevice` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`; `crate::native_ffi::GgmlDevice` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`; `crate::native_ffi` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::integrity — `crates/whisper-worker-gpu/src/integrity.rs`

Réexporte la vérification SHA-256 partagée. `crates/whisper-worker-gpu/src/integrity.rs:1`

Mesure : 1 production / 0 tests / 1 physiques. `crates/whisper-worker-gpu/src/integrity.rs:1`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/integrity.rs:1`.

Fonctions avec corps : aucune relevée dans `crates/whisper-worker-gpu/src/integrity.rs:1`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/integrity.rs:1`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/integrity.rs:1`.

Utilisé par, via ces imports : tests : `crate::diagnostics::tests` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:2`<br>hors tests : `crate::gpu_commands` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::ipc::tests — `crates/whisper-worker-gpu/src/ipc/tests.rs`

Tests ou supports : définit `rejects_oversized_and_incomplete_frames_before_deserialization` `crates/whisper-worker-gpu/src/ipc/tests.rs:4-20`, `writer_accepts_limit_and_rejects_limit_plus_one` `crates/whisper-worker-gpu/src/ipc/tests.rs:23-34`.

Mesure : 0 production / 34 tests / 34 physiques. `crates/whisper-worker-gpu/src/ipc/tests.rs:1-34`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/ipc/tests.rs:1-34`.

Fonctions avec corps : `rejects_oversized_and_incomplete_frames_before_deserialization` [tests] `crates/whisper-worker-gpu/src/ipc/tests.rs:4-20`; `writer_accepts_limit_and_rejects_limit_plus_one` [tests] `crates/whisper-worker-gpu/src/ipc/tests.rs:23-34`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/ipc/tests.rs:1-34`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/ipc/tests.rs:1-34`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc — `crates/whisper-worker-gpu/src/ipc.rs`

Adapte les trames partagées à l’observateur local et définit Incoming. `crates/whisper-worker-gpu/src/ipc.rs:1-30`

Mesure : 26 production / 4 tests / 30 physiques. `crates/whisper-worker-gpu/src/ipc.rs:1-30`

Types : `Incoming` (type, hors tests) `crates/whisper-worker-gpu/src/ipc.rs:26-27`.

Fonctions avec corps : `observer` `crates/whisper-worker-gpu/src/ipc.rs:7-16`; `read_frame` `crates/whisper-worker-gpu/src/ipc.rs:18-20`; `write_frame` `crates/whisper-worker-gpu/src/ipc.rs:22-24`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_worker_common::ipc::MAX_FRAME_BYTES;` `crates/whisper-worker-gpu/src/ipc.rs:5`; `pub fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead) -> io::Result<Option<T>> {` `crates/whisper-worker-gpu/src/ipc.rs:18`; `pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {` `crates/whisper-worker-gpu/src/ipc.rs:22`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/ipc.rs:1-30`.

Utilisé par, via ces imports : tests : `crate::diagnostics::tests` `crates/whisper-worker-gpu/src/diagnostics/tests.rs:2`<br>hors tests : `crate::diagnostics` `crates/whisper-worker-gpu/src/diagnostics.rs:1`<br>hors tests : `crate::gpu_commands` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:2`<br>hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::worker` `crates/whisper-worker-gpu/src/worker.rs:1`.

### crate — `crates/whisper-worker-gpu/src/main.rs`

Délègue le point d’entrée à startup::run. `crates/whisper-worker-gpu/src/main.rs:1-20`

Mesure : 20 production / 0 tests / 20 physiques. `crates/whisper-worker-gpu/src/main.rs:1-20`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/main.rs:1-20`.

Fonctions avec corps : `main` `crates/whisper-worker-gpu/src/main.rs:18-20`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/main.rs:1-20`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/main.rs:1-20`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::memory_qualification — `crates/whisper-worker-gpu/src/memory_qualification.rs`

Écrit les mesures mémoire conditionnelles du worker. `crates/whisper-worker-gpu/src/memory_qualification.rs:1-92`

Mesure : 92 production / 0 tests / 92 physiques. `crates/whisper-worker-gpu/src/memory_qualification.rs:1-92`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/memory_qualification.rs:1-92`.

Fonctions avec corps : `record` `crates/whisper-worker-gpu/src/memory_qualification.rs:12-62`; `process_memory_sample` `crates/whisper-worker-gpu/src/memory_qualification.rs:64-92`.

Déclarations `pub` (visibilité sous conditions) : `pub fn record(stage: &str, details: Value) {` `crates/whisper-worker-gpu/src/memory_qualification.rs:12`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/memory_qualification.rs:1-92`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:2`<br>hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:2`.

### crate::native_engine — `crates/whisper-worker-gpu/src/native_engine.rs`

Charge whisper-rs et transforme ses résultats de transcription. `crates/whisper-worker-gpu/src/native_engine.rs:1-120`

Mesure : 120 production / 0 tests / 120 physiques. `crates/whisper-worker-gpu/src/native_engine.rs:1-120`

Types : `GpuEngine` (struct, hors tests) `crates/whisper-worker-gpu/src/native_engine.rs:5-10`; `TextSegment` (struct, hors tests) `crates/whisper-worker-gpu/src/native_engine.rs:12-16`.

Fonctions avec corps : `attest` `crates/whisper-worker-gpu/src/native_engine.rs:19-37`; `load` `crates/whisper-worker-gpu/src/native_engine.rs:39-63`; `matches_device` `crates/whisper-worker-gpu/src/native_engine.rs:65-67`; `transcribe` `crates/whisper-worker-gpu/src/native_engine.rs:69-119`.

Déclarations `pub` (visibilité sous conditions) : `pub const MAX_LIVE_WINDOW_SAMPLES: usize = 128_000;` `crates/whisper-worker-gpu/src/native_engine.rs:3`; `pub struct GpuEngine {` `crates/whisper-worker-gpu/src/native_engine.rs:5`; `pub struct TextSegment {` `crates/whisper-worker-gpu/src/native_engine.rs:12`; `pub fn attest(&self) -> Result<(), String> {` `crates/whisper-worker-gpu/src/native_engine.rs:19`; `pub fn load(` `crates/whisper-worker-gpu/src/native_engine.rs:39`; `pub fn matches_device(&self, whisper_gpu_index: u32, expected_backend: &str) -> bool {` `crates/whisper-worker-gpu/src/native_engine.rs:65`; `pub fn transcribe(` `crates/whisper-worker-gpu/src/native_engine.rs:69`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/native_engine.rs:1-120`.

Utilisé par, via ces imports : hors tests : `crate::engine_cache` `crates/whisper-worker-gpu/src/engine_cache.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::native_ffi — `crates/whisper-worker-gpu/src/native_ffi.rs`

Interroge CUDA/GGML et corrèle identités natives/Windows et traces GPU. `crates/whisper-worker-gpu/src/native_ffi.rs:1-623`

Mesure : 557 production / 66 tests / 623 physiques. `crates/whisper-worker-gpu/src/native_ffi.rs:1-623`

Types : `NativeLogPhase` (struct, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:9-14`; `BackendRegistryEntry` (struct, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:120-125`; `Driver` (struct, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:325`; `Init` (type, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:356`; `Count` (type, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:357`; `Get` (type, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:358`; `Luid` (type, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:359`; `Pci` (type, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:360`; `Capability` (type, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:361`; `CudaDevice` (struct, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:428-433`; `GgmlDevice` (struct, hors tests) `crates/whisper-worker-gpu/src/native_ffi.rs:436-440`.

Fonctions avec corps : `native_log_phase` `crates/whisper-worker-gpu/src/native_ffi.rs:18-20`; `begin_native_gpu_phase` `crates/whisper-worker-gpu/src/native_ffi.rs:22-32`; `finish_native_gpu_phase` `crates/whisper-worker-gpu/src/native_ffi.rs:34-43`; `validate_native_gpu_log` `crates/whisper-worker-gpu/src/native_ffi.rs:45-80`; `install_native_log_callback` `crates/whisper-worker-gpu/src/native_ffi.rs:82-93`; `native_log_callback` `crates/whisper-worker-gpu/src/native_ffi.rs:95-117`; `whisper_gpu_position` `crates/whisper-worker-gpu/src/native_ffi.rs:127-146`; `validate_selected_cuda_registry_device` `crates/whisper-worker-gpu/src/native_ffi.rs:148-164`; `native_gpu_attestation_requires_a_fresh_matching_log` [tests] `crates/whisper-worker-gpu/src/native_ffi.rs:171-189`; `native_gpu_attestation_rejects_failure_after_using_backend` [tests] `crates/whisper-worker-gpu/src/native_ffi.rs:192-195`; `whisper_position_counts_gpu_and_integrated_gpu_not_cuda_ordinals` [tests] `crates/whisper-worker-gpu/src/native_ffi.rs:198-230`; `whisper_gpu_device_index` `crates/whisper-worker-gpu/src/native_ffi.rs:233-275`; `normalize_pci` `crates/whisper-worker-gpu/src/native_ffi.rs:277-283`; `cuda_backend_available` `crates/whisper-worker-gpu/src/native_ffi.rs:285-312`; `cuda_devices` `crates/whisper-worker-gpu/src/native_ffi.rs:315-420`; `drop` `crates/whisper-worker-gpu/src/native_ffi.rs:327-332`; `symbol` `crates/whisper-worker-gpu/src/native_ffi.rs:340-355`; `cuda_devices` `crates/whisper-worker-gpu/src/native_ffi.rs:423-425`; `ggml_devices` `crates/whisper-worker-gpu/src/native_ffi.rs:442-495`; `verify_windows_identity` `crates/whisper-worker-gpu/src/native_ffi.rs:498-540`; `verify_windows_identity` `crates/whisper-worker-gpu/src/native_ffi.rs:543-547`; `device_interface_instance_id` `crates/whisper-worker-gpu/src/native_ffi.rs:550-623`.

Déclarations `pub` (visibilité sous conditions) : `pub fn begin_native_gpu_phase() -> Result<(), String> {` `crates/whisper-worker-gpu/src/native_ffi.rs:22`; `pub fn finish_native_gpu_phase(expected_backend: &str) -> Result<(), String> {` `crates/whisper-worker-gpu/src/native_ffi.rs:34`; `pub fn whisper_gpu_device_index(` `crates/whisper-worker-gpu/src/native_ffi.rs:233`; `pub fn cuda_backend_available() -> bool {` `crates/whisper-worker-gpu/src/native_ffi.rs:285`; `pub fn cuda_devices() -> Result<Vec<CudaDevice>, String> {` `crates/whisper-worker-gpu/src/native_ffi.rs:315`; `pub fn cuda_devices() -> Result<Vec<CudaDevice>, String> {` `crates/whisper-worker-gpu/src/native_ffi.rs:423`; `pub struct CudaDevice {` `crates/whisper-worker-gpu/src/native_ffi.rs:428`; `pub struct GgmlDevice {` `crates/whisper-worker-gpu/src/native_ffi.rs:436`; `pub fn ggml_devices() -> Vec<GgmlDevice> {` `crates/whisper-worker-gpu/src/native_ffi.rs:442`; `pub fn verify_windows_identity(` `crates/whisper-worker-gpu/src/native_ffi.rs:498`; `pub fn verify_windows_identity(` `crates/whisper-worker-gpu/src/native_ffi.rs:543`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/native_ffi.rs:1-623`.

Utilisé par, via ces imports : hors tests : `crate::engine_cache` `crates/whisper-worker-gpu/src/engine_cache.rs:1`<br>hors tests : `crate::gpu_commands` `crates/whisper-worker-gpu/src/gpu_commands.rs:1`<br>tests : `crate::gpu_qualification` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`<br>tests : `crate::gpu_qualification` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`<br>tests : `crate::gpu_qualification` `crates/whisper-worker-gpu/src/gpu_qualification.rs:137`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

#### Inline `tests`

Tests/auxiliaires : `native_gpu_attestation_requires_a_fresh_matching_log` `crates/whisper-worker-gpu/src/native_ffi.rs:171-189`, `native_gpu_attestation_rejects_failure_after_using_backend` `crates/whisper-worker-gpu/src/native_ffi.rs:192-195`, `whisper_position_counts_gpu_and_integrated_gpu_not_cuda_ordinals` `crates/whisper-worker-gpu/src/native_ffi.rs:198-230`. `crates/whisper-worker-gpu/src/native_ffi.rs:166-231`

Mesure locale : 0 production / 66 tests ; sous-ensemble non additionnable. `crates/whisper-worker-gpu/src/native_ffi.rs:166-231`

Types locaux : aucun relevé dans `crates/whisper-worker-gpu/src/native_ffi.rs:166-231`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-gpu/src/native_ffi.rs:166-231`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::qualification — `crates/whisper-worker-gpu/src/qualification.rs`

Implémente résultats/barrières de campagne L04 conditionnés par recette-hooks. `crates/whisper-worker-gpu/src/qualification.rs:1-307`

Mesure : 307 production / 0 tests / 307 physiques. `crates/whisper-worker-gpu/src/qualification.rs:1-307`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/qualification.rs:1-307`.

Fonctions avec corps : `l04_campaign_worker_result` `crates/whisper-worker-gpu/src/qualification.rs:8-71`; `l04_campaign_worker_result` `crates/whisper-worker-gpu/src/qualification.rs:74-82`; `l04_campaign_gate` `crates/whisper-worker-gpu/src/qualification.rs:85-297`; `l04_campaign_gate` `crates/whisper-worker-gpu/src/qualification.rs:300-307`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/qualification.rs:1-307`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/qualification.rs:1-307`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

### crate::session::control::tests — `crates/whisper-worker-gpu/src/session/control/tests.rs`

Tests ou supports : définit `empty_and_disconnected_channels_do_not_stop_the_session` `crates/whisper-worker-gpu/src/session/control/tests.rs:4-11`, `wrong_stop_is_ignored_and_matching_stop_is_consumed` `crates/whisper-worker-gpu/src/session/control/tests.rs:14-27`.

Mesure : 0 production / 47 tests / 47 physiques. `crates/whisper-worker-gpu/src/session/control/tests.rs:1-47`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/session/control/tests.rs:1-47`.

Fonctions avec corps : `empty_and_disconnected_channels_do_not_stop_the_session` [tests] `crates/whisper-worker-gpu/src/session/control/tests.rs:4-11`; `wrong_stop_is_ignored_and_matching_stop_is_consumed` [tests] `crates/whisper-worker-gpu/src/session/control/tests.rs:14-27`; `wrong_protocol_emits_correlated_failure_before_stopping` [tests] `crates/whisper-worker-gpu/src/session/control/tests.rs:30-47`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/session/control/tests.rs:1-47`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/session/control/tests.rs:1-47`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::control — `crates/whisper-worker-gpu/src/session/control.rs`

Draine les commandes d’arrêt et contrôle leur identité. `crates/whisper-worker-gpu/src/session/control.rs:1-44`

Mesure : 42 production / 2 tests / 44 physiques. `crates/whisper-worker-gpu/src/session/control.rs:1-44`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/session/control.rs:1-44`.

Fonctions avec corps : `take_stop` `crates/whisper-worker-gpu/src/session/control.rs:3-41`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/session/control.rs:1-44`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/session/control.rs:1-44`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::decode — `crates/whisper-worker-gpu/src/session/decode.rs`

Traite DecodeBlock et émet les blocs PCM. `crates/whisper-worker-gpu/src/session/decode.rs:1-148`

Mesure : 148 production / 0 tests / 148 physiques. `crates/whisper-worker-gpu/src/session/decode.rs:1-148`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/session/decode.rs:1-148`.

Fonctions avec corps : `handle_decode` `crates/whisper-worker-gpu/src/session/decode.rs:3-91`; `run_decode_service` `crates/whisper-worker-gpu/src/session/decode.rs:93-148`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/session/decode.rs:1-148`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/session/decode.rs:1-148`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::import — `crates/whisper-worker-gpu/src/session/import.rs`

Vérifie source/modèle et exécute la transcription import par fenêtres. `crates/whisper-worker-gpu/src/session/import.rs:1-371`

Mesure : 371 production / 0 tests / 371 physiques. `crates/whisper-worker-gpu/src/session/import.rs:1-371`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/session/import.rs:1-371`.

Fonctions avec corps : `run_import` `crates/whisper-worker-gpu/src/session/import.rs:2-371`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/session/import.rs:1-371`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/session/import.rs:1-371`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::live — `crates/whisper-worker-gpu/src/session/live.rs`

Exécute les commandes audio live et émet segments, paquets et fins de fenêtres. `crates/whisper-worker-gpu/src/session/live.rs:1-254`

Mesure : 254 production / 0 tests / 254 physiques. `crates/whisper-worker-gpu/src/session/live.rs:1-254`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/session/live.rs:1-254`.

Fonctions avec corps : `run_live_service` `crates/whisper-worker-gpu/src/session/live.rs:3-254`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/session/live.rs:1-254`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/session/live.rs:1-254`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session — `crates/whisper-worker-gpu/src/session/mod.rs`

Vérifie le protocole et route les commandes import/live/décodage. `crates/whisper-worker-gpu/src/session/mod.rs:1-335`

Mesure : 335 production / 0 tests / 335 physiques. `crates/whisper-worker-gpu/src/session/mod.rs:1-335`

Types : `ImportRequest` (struct, hors tests) `crates/whisper-worker-gpu/src/session/mod.rs:37-48`.

Fonctions avec corps : `run_session` `crates/whisper-worker-gpu/src/session/mod.rs:50-335`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/session/mod.rs:1-335`.

Dépend de, via `use crate::` : hors tests : `crate::memory_qualification` → `crate::memory_qualification` `crates/whisper-worker-gpu/src/session/mod.rs:2`<br>hors tests : `crate::decoder` → `crate::decoder` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::diagnostics::emit_failure` → `crate::diagnostics` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::diagnostics::emit_gpu_initialization_failure` → `crate::diagnostics` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::diagnostics::emit_gpu_revalidation_failure` → `crate::diagnostics` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::diagnostics` → `crate::diagnostics` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::encoder` → `crate::encoder` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::engine_cache::CachedGpuEngine` → `crate::engine_cache` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::engine_cache::cached_gpu_engine` → `crate::engine_cache` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::engine_cache` → `crate::engine_cache` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::gpu_attestation` → `crate::gpu_attestation` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::gpu_commands` → `crate::gpu_commands` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::integrity::verify_sha256` → `crate::integrity` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::ipc::Incoming` → `crate::ipc` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::ipc::write_frame` → `crate::ipc` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::ipc` → `crate::ipc` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::native_engine::self` → `crate::native_engine` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::native_engine::GpuEngine` → `crate::native_engine` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::native_engine` → `crate::native_engine` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::native_ffi` → `crate::native_ffi` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::qualification::l04_campaign_gate` → `crate::qualification` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::qualification::l04_campaign_worker_result` → `crate::qualification` `crates/whisper-worker-gpu/src/session/mod.rs:3`<br>hors tests : `crate::qualification` → `crate::qualification` `crates/whisper-worker-gpu/src/session/mod.rs:3`.

Utilisé par, via ces imports : hors tests : `crate::worker` `crates/whisper-worker-gpu/src/worker.rs:1`.

### crate::startup — `crates/whisper-worker-gpu/src/startup.rs`

Ouvre stdin/stdout, crée le canal et lance le thread lecteur IPC. `crates/whisper-worker-gpu/src/startup.rs:1-82`

Mesure : 82 production / 0 tests / 82 physiques. `crates/whisper-worker-gpu/src/startup.rs:1-82`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/startup.rs:1-82`.

Fonctions avec corps : `run` `crates/whisper-worker-gpu/src/startup.rs:14-39`; `input_loop` `crates/whisper-worker-gpu/src/startup.rs:41-82`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/startup.rs:1-82`.

Dépend de, via `use crate::` : hors tests : `crate::ipc` → `crate::ipc` `crates/whisper-worker-gpu/src/startup.rs:2`<br>hors tests : `crate::memory_qualification` → `crate::memory_qualification` `crates/whisper-worker-gpu/src/startup.rs:2`<br>hors tests : `crate::ipc::Incoming` → `crate::ipc` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::ipc::read_frame` → `crate::ipc` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::ipc::write_frame` → `crate::ipc` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::ipc` → `crate::ipc` `crates/whisper-worker-gpu/src/startup.rs:3`<br>hors tests : `crate::worker::run` → `crate::worker` `crates/whisper-worker-gpu/src/startup.rs:3`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker::tests — `crates/whisper-worker-gpu/src/worker/tests.rs`

Tests ou supports : définit `idle_worker_shutdown_exits_without_an_error_frame` `crates/whisper-worker-gpu/src/worker/tests.rs:6-14`.

Mesure : 0 production / 14 tests / 14 physiques. `crates/whisper-worker-gpu/src/worker/tests.rs:1-14`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/worker/tests.rs:1-14`.

Fonctions avec corps : `idle_worker_shutdown_exits_without_an_error_frame` [tests] `crates/whisper-worker-gpu/src/worker/tests.rs:6-14`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/worker/tests.rs:1-14`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/src/worker/tests.rs:1-14`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker — `crates/whisper-worker-gpu/src/worker.rs`

Boucle sur les sessions et conserve le cache moteur entre demandes. `crates/whisper-worker-gpu/src/worker.rs:1-40`

Mesure : 38 production / 2 tests / 40 physiques. `crates/whisper-worker-gpu/src/worker.rs:1-40`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/src/worker.rs:1-40`.

Fonctions avec corps : `run` `crates/whisper-worker-gpu/src/worker.rs:5-37`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/src/worker.rs:1-40`.

Dépend de, via `use crate::` : hors tests : `crate::engine_cache::CachedGpuEngine` → `crate::engine_cache` `crates/whisper-worker-gpu/src/worker.rs:1`<br>hors tests : `crate::ipc::Incoming` → `crate::ipc` `crates/whisper-worker-gpu/src/worker.rs:1`<br>hors tests : `crate::session::run_session` → `crate::session` `crates/whisper-worker-gpu/src/worker.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::startup` `crates/whisper-worker-gpu/src/startup.rs:3`.

### integration::gpu_selection_native — `crates/whisper-worker-gpu/tests/gpu_selection_native.rs`

Tests ou supports : définit `linked_gpu_worker_answers_a_native_cuda_probe_without_loading_a_model` `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:8-31`.

Mesure : 0 production / 31 tests / 31 physiques. `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:1-31`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:1-31`.

Fonctions avec corps : `linked_gpu_worker_answers_a_native_cuda_probe_without_loading_a_model` [tests] `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:8-31`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:1-31`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/tests/gpu_selection_native.rs:1-31`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::support::worker / integration::worker_contract::support — `crates/whisper-worker-gpu/tests/support/worker.rs`

Tests ou supports : définit `deadline_child_fixture` `crates/whisper-worker-gpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-gpu/tests/support/worker.rs:226-259`.

Mesure : 0 production / 282 tests / 282 physiques. `crates/whisper-worker-gpu/tests/support/worker.rs:1-282`

Types : `BoundedChild` (struct, tests) `crates/whisper-worker-gpu/tests/support/worker.rs:16-22`.

Fonctions avec corps : `spawn` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:25-50`; `id` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:52-54`; `stdin` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:55-57`; `stdout` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:58-60`; `stderr` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:61-63`; `remaining` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:64-66`; `timed_out` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:67-69`; `wait` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:71-106`; `drop` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:110-131`; `finish_reader` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:134-143`; `output` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:145-210`; `deadline_child_fixture` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:217-223`; `deadline_stops_child_after_expected_output` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:226-259`; `timed_out_output_retains_the_captured_stdout` [tests] `crates/whisper-worker-gpu/tests/support/worker.rs:262-281`.

Déclarations `pub` (visibilité sous conditions) : `pub const CONTROLLED_TIMEOUT: Duration = Duration::from_secs(300);` `crates/whisper-worker-gpu/tests/support/worker.rs:13`; `pub const NATIVE_TIMEOUT: Duration = Duration::from_secs(600);` `crates/whisper-worker-gpu/tests/support/worker.rs:14`; `pub struct BoundedChild {` `crates/whisper-worker-gpu/tests/support/worker.rs:16`; `pub fn spawn(command: &mut Command, timeout: Duration) -> Self {` `crates/whisper-worker-gpu/tests/support/worker.rs:25`; `pub fn id(&self) -> u32 {` `crates/whisper-worker-gpu/tests/support/worker.rs:52`; `pub fn stdin(&self) -> ChildStdin {` `crates/whisper-worker-gpu/tests/support/worker.rs:55`; `pub fn stdout(&self) -> ChildStdout {` `crates/whisper-worker-gpu/tests/support/worker.rs:58`; `pub fn stderr(&self) -> ChildStderr {` `crates/whisper-worker-gpu/tests/support/worker.rs:61`; `pub fn remaining(&self) -> Duration {` `crates/whisper-worker-gpu/tests/support/worker.rs:64`; `pub fn timed_out(&self) -> bool {` `crates/whisper-worker-gpu/tests/support/worker.rs:67`; `pub fn wait(&self) -> Result<ExitStatus, String> {` `crates/whisper-worker-gpu/tests/support/worker.rs:71`; `pub fn finish_reader<T>(reader: JoinHandle<T>, child: &BoundedChild) -> T {` `crates/whisper-worker-gpu/tests/support/worker.rs:134`; `pub fn output(command: &mut Command, input: Option<Vec<u8>>, timeout: Duration) -> Output {` `crates/whisper-worker-gpu/tests/support/worker.rs:145`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/tests/support/worker.rs:1-282`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `deadline_child_fixture` `crates/whisper-worker-gpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-gpu/tests/support/worker.rs:226-259`, `timed_out_output_retains_the_captured_stdout` `crates/whisper-worker-gpu/tests/support/worker.rs:262-281`. `crates/whisper-worker-gpu/tests/support/worker.rs:212-282`

Mesure locale : 0 production / 71 tests ; sous-ensemble non additionnable. `crates/whisper-worker-gpu/tests/support/worker.rs:212-282`

Types locaux : aucun relevé dans `crates/whisper-worker-gpu/tests/support/worker.rs:212-282`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-gpu/tests/support/worker.rs:212-282`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### integration::worker_contract — `crates/whisper-worker-gpu/tests/worker_contract.rs`

Tests ou supports : définit `turbo_idle_and_refusal_contract` `crates/whisper-worker-gpu/tests/worker_contract.rs:29-63`, `gpu_identity_contract` `crates/whisper-worker-gpu/tests/worker_contract.rs:66-103`.

Mesure : 0 production / 500 tests / 500 physiques. `crates/whisper-worker-gpu/tests/worker_contract.rs:1-500`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-gpu/tests/worker_contract.rs:1-500`.

Fonctions avec corps : `run_command` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:18-26`; `turbo_idle_and_refusal_contract` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:29-63`; `gpu_identity_contract` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:66-103`; `turbo_reusable_import_contract` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:107-116`; `model_hash` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:118-130`; `send_command` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:132-136`; `live_case` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:138-266`; `turbo_live_stop_and_shutdown_contract` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:270-284`; `hash_file` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:305-317`; `write_silent_wav` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:319-337`; `send` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:339-343`; `await_end` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:345-395`; `run_for_exe` [tests] `crates/whisper-worker-gpu/tests/worker_contract.rs:397-499`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-gpu/tests/worker_contract.rs:1-500`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-gpu/tests/worker_contract.rs:1-500`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `gpu_reuse`

Tests/auxiliaires : `hash_file` `crates/whisper-worker-gpu/tests/worker_contract.rs:305-317`, `write_silent_wav` `crates/whisper-worker-gpu/tests/worker_contract.rs:319-337`, `send` `crates/whisper-worker-gpu/tests/worker_contract.rs:339-343`, `await_end` `crates/whisper-worker-gpu/tests/worker_contract.rs:345-395`. `crates/whisper-worker-gpu/tests/worker_contract.rs:286-500`

Mesure locale : 0 production / 215 tests ; sous-ensemble non additionnable. `crates/whisper-worker-gpu/tests/worker_contract.rs:286-500`

Types locaux : aucun relevé dans `crates/whisper-worker-gpu/tests/worker_contract.rs:286-500`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-gpu/tests/worker_contract.rs:286-500`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 0 | 2 |
| cfg_feature | 20 | 0 |
| cfg_test | 0 | 11 |
| command_new | 1 | 6 |
| expect | 3 | 18 |
| thread_spawn | 1 | 7 |
| unsafe | 39 | 0 |
| unwrap | 1 | 71 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 15 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 12 sur plages hors tests et 3 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::field_reassign_with_default | 4 | 0 |
| clippy::ok_expect | 0 | 1 |
| clippy::too_many_arguments | 2 | 0 |
| clippy::too_many_lines | 6 | 1 |
| dead_code | 0 | 1 |
