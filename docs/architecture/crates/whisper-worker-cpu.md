Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-worker-cpu

## Rôle et type

Lit les commandes IPC et exécute import, live ou décodage. `crates/whisper-worker-cpu/src/startup.rs:1-86` ; `crates/whisper-worker-cpu/src/session/mod.rs:1-169` ; `crates/whisper-worker-cpu/src/engine_cache.rs:1-23` ; `crates/whisper-worker-cpu/src/native_engine.rs:1-196` ; `crates/whisper-worker-cpu/src/session/import.rs:1-293`

Charge ou réutilise un moteur whisper-rs CPU et émet ses résultats par IPC. `crates/whisper-worker-cpu/src/startup.rs:1-86` ; `crates/whisper-worker-cpu/src/session/mod.rs:1-169` ; `crates/whisper-worker-cpu/src/engine_cache.rs:1-23` ; `crates/whisper-worker-cpu/src/native_engine.rs:1-196` ; `crates/whisper-worker-cpu/src/session/import.rs:1-293`

Type : **Worker binaire CPU**. `crates/whisper-worker-cpu/src/startup.rs:1-86` ; `crates/whisper-worker-cpu/src/session/mod.rs:1-169` ; `crates/whisper-worker-cpu/src/engine_cache.rs:1-23` ; `crates/whisper-worker-cpu/src/native_engine.rs:1-196` ; `crates/whisper-worker-cpu/src/session/import.rs:1-293`

Mesures : **1751 production / 958 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-worker-common | interne | dependencies | {"workspace": true} | `crates/whisper-worker-cpu/Cargo.toml:15` |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-worker-cpu/Cargo.toml:16` |
| serde_json | externe | dependencies | {"workspace": true} | `crates/whisper-worker-cpu/Cargo.toml:17` |
| serde | externe | dependencies | {"version": "=1.0.228", "features": ["derive"]} | `crates/whisper-worker-cpu/Cargo.toml:18` |
| sha2 | externe | dependencies | "=0.10.9" | `crates/whisper-worker-cpu/Cargo.toml:19` |
| whisper-rs | externe | dependencies | {"version": "=0.16.0", "default-features": false} | `crates/whisper-worker-cpu/Cargo.toml:20` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-worker-cpu/Cargo.toml:1-23`.

## Interfaces publiques et usages intercrates

[Non vérifié] Aucun import consommateur d’une autre crate relevé ; les autres usages ne sont pas exclus.

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-worker-cpu/src/main.rs:1-19`. Imports textuels, pas un graphe de symboles résolu.

Interface entre processus : `crates/whisper-core/src/ipc.rs:1-343`.

Cible de build partagée : `scripts/recette-build.rs:1-7` (7 lignes hors tests, comptées une seule fois hors des totaux par crate). Elle refuse recette-hooks en release. Déclaration : `crates/whisper-worker-cpu/Cargo.toml:7`.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | `crates/whisper-worker-cpu/src/memory_qualification.rs:3` ; `crates/whisper-worker-cpu/src/native_engine.rs:25` ; `crates/whisper-worker-cpu/src/qualification.rs:31` ; `crates/whisper-worker-cpu/src/session/import.rs:65` |
| Processus | `crates/whisper-worker-cpu/src/memory_qualification.rs:71` |
| Threads/canaux | `crates/whisper-worker-cpu/src/native_engine.rs:114` ; `crates/whisper-worker-cpu/src/qualification.rs:294` ; `crates/whisper-worker-cpu/src/session/control.rs:37` ; `crates/whisper-worker-cpu/src/session/mod.rs:17` ; `crates/whisper-worker-cpu/src/startup.rs:12` ; `crates/whisper-worker-cpu/src/worker.rs:2` |
| Entrées/sorties standard | `crates/whisper-worker-cpu/src/memory_qualification.rs:73` ; `crates/whisper-worker-cpu/src/startup.rs:21` |
| GPU/natif | `crates/whisper-worker-cpu/src/native_engine.rs:2` |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate::decoder::tests | Tests ou supports : définit `inaccessible_source_propagates_error` `crates/whisper-worker-cpu/src/decoder/tests.rs:4-11`. | 0 | 11 |
| crate::decoder | Adapte les fonctions de décodage partagées à l’observateur local. `crates/whisper-worker-cpu/src/decoder.rs:1-29` | 27 | 2 |
| crate::diagnostics | Émet les événements IPC de défaillance. `crates/whisper-worker-cpu/src/diagnostics.rs:1-24` | 24 | 0 |
| crate::encoder::tests | Tests ou supports : définit `capture_encoder_baseline` `crates/whisper-worker-cpu/src/encoder/tests.rs:5-34`. | 0 | 34 |
| crate::encoder | Réexporte l’encodeur MP3 partagé. `crates/whisper-worker-cpu/src/encoder.rs:1-4` | 2 | 2 |
| crate::engine_cache | Vérifie l’identité demandée avant réutilisation ou chargement du moteur. `crates/whisper-worker-cpu/src/engine_cache.rs:1-23` | 23 | 0 |
| crate::integrity | Réexporte la vérification SHA-256 partagée. `crates/whisper-worker-cpu/src/integrity.rs:1` | 1 | 0 |
| crate::ipc::tests | Tests ou supports : définit `rejects_oversized_and_incomplete_frames_before_deserialization` `crates/whisper-worker-cpu/src/ipc/tests.rs:4-20`, `rejects_serialized_frames_over_one_mibibyte` `crates/whisper-worker-cpu/src/ipc/tests.rs:23-29`. | 0 | 29 |
| crate::ipc | Adapte les trames partagées à l’observateur local et définit Incoming. `crates/whisper-worker-cpu/src/ipc.rs:1-30` | 26 | 4 |
| crate::ipc_output::tests | Tests ou supports : définit `writer_accepts_limit_and_rejects_limit_plus_one_without_growing_past_it` `crates/whisper-worker-cpu/src/ipc_output/tests.rs:4-15`. | 0 | 15 |
| crate::ipc_output | Écrit et vide les trames de sortie IPC. `crates/whisper-worker-cpu/src/ipc_output.rs:1-14` | 10 | 4 |
| crate | Délègue le point d’entrée à startup::run. `crates/whisper-worker-cpu/src/main.rs:1-19` | 19 | 0 |
| crate::memory_qualification | Écrit les mesures mémoire conditionnelles du worker. `crates/whisper-worker-cpu/src/memory_qualification.rs:1-92` | 92 | 0 |
| crate::native_engine | Charge whisper-rs et transforme ses résultats de transcription. `crates/whisper-worker-cpu/src/native_engine.rs:1-196` | 196 | 0 |
| crate::qualification | Implémente résultats/barrières de campagne L04 conditionnés par recette-hooks. `crates/whisper-worker-cpu/src/qualification.rs:1-307` | 307 | 0 |
| crate::session::control::tests | Tests ou supports : définit `empty_and_disconnected_channels_do_not_stop_the_session` `crates/whisper-worker-cpu/src/session/control/tests.rs:4-11`, `wrong_stop_is_ignored_and_matching_stop_is_consumed` `crates/whisper-worker-cpu/src/session/control/tests.rs:14-27`. | 0 | 47 |
| crate::session::control | Draine les commandes d’arrêt et contrôle leur identité. `crates/whisper-worker-cpu/src/session/control.rs:1-44` | 42 | 2 |
| crate::session::decode | Traite DecodeBlock et émet les blocs PCM. `crates/whisper-worker-cpu/src/session/decode.rs:1-148` | 148 | 0 |
| crate::session::import | Vérifie source/modèle et exécute la transcription import par fenêtres. `crates/whisper-worker-cpu/src/session/import.rs:1-293` | 293 | 0 |
| crate::session::live | Exécute les commandes audio live et émet segments, paquets et fins de fenêtres. `crates/whisper-worker-cpu/src/session/live.rs:1-254` | 254 | 0 |
| crate::session | Vérifie le protocole et route les commandes import/live/décodage. `crates/whisper-worker-cpu/src/session/mod.rs:1-169` | 169 | 0 |
| crate::startup | Ouvre stdin/stdout, crée le canal et lance le thread lecteur IPC. `crates/whisper-worker-cpu/src/startup.rs:1-86` | 86 | 0 |
| crate::worker::tests | Tests ou supports : définit `idle_worker_shutdown_exits_without_an_error_frame` `crates/whisper-worker-cpu/src/worker/tests.rs:6-14`. | 0 | 14 |
| crate::worker | Boucle sur les sessions et conserve le cache moteur entre demandes. `crates/whisper-worker-cpu/src/worker.rs:1-34` | 32 | 2 |
| integration::live_worker | Tests ou supports : définit `live_worker_rejects_an_unverified_model_before_reporting_ready` `crates/whisper-worker-cpu/tests/live_worker.rs:11-44`. | 0 | 44 |
| integration::reusable_imports / integration::worker_contract::reusable_imports | Tests ou supports : définit `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache` `crates/whisper-worker-cpu/tests/reusable_imports.rs:114-116`. | 0 | 220 |
| integration::reusable_imports::support / integration::support::worker / integration::worker_contract::reusable_imports::support / integration::worker_contract::support | Tests ou supports : définit `deadline_child_fixture` `crates/whisper-worker-cpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-cpu/tests/support/worker.rs:226-259`. | 0 | 282 |
| integration::worker_contract | Tests ou supports : définit `turbo_idle_and_refusal_contract` `crates/whisper-worker-cpu/tests/worker_contract.rs:31-65`, `turbo_reusable_import_contract` `crates/whisper-worker-cpu/tests/worker_contract.rs:69-78`. | 0 | 246 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| tests | externe | tests | crates/whisper-worker-cpu/src/decoder/tests.rs | `crates/whisper-worker-cpu/src/decoder.rs:28-29` |
| tests | externe | tests | crates/whisper-worker-cpu/src/encoder/tests.rs | `crates/whisper-worker-cpu/src/encoder.rs:3-4` |
| tests | externe | tests | crates/whisper-worker-cpu/src/ipc/tests.rs | `crates/whisper-worker-cpu/src/ipc.rs:29-30` |
| tests | externe | tests | crates/whisper-worker-cpu/src/ipc_output/tests.rs | `crates/whisper-worker-cpu/src/ipc_output.rs:13-14` |
| decoder | externe | hors tests / cfg | crates/whisper-worker-cpu/src/decoder.rs | `crates/whisper-worker-cpu/src/main.rs:1` |
| diagnostics | externe | hors tests / cfg | crates/whisper-worker-cpu/src/diagnostics.rs | `crates/whisper-worker-cpu/src/main.rs:2` |
| encoder | externe | hors tests / cfg | crates/whisper-worker-cpu/src/encoder.rs | `crates/whisper-worker-cpu/src/main.rs:3` |
| engine_cache | externe | hors tests / cfg | crates/whisper-worker-cpu/src/engine_cache.rs | `crates/whisper-worker-cpu/src/main.rs:4` |
| integrity | externe | hors tests / cfg | crates/whisper-worker-cpu/src/integrity.rs | `crates/whisper-worker-cpu/src/main.rs:5` |
| ipc | externe | hors tests / cfg | crates/whisper-worker-cpu/src/ipc.rs | `crates/whisper-worker-cpu/src/main.rs:6-7` |
| ipc_output | externe | hors tests / cfg | crates/whisper-worker-cpu/src/ipc_output.rs | `crates/whisper-worker-cpu/src/main.rs:8` |
| memory_qualification | externe | hors tests / cfg | crates/whisper-worker-cpu/src/memory_qualification.rs | `crates/whisper-worker-cpu/src/main.rs:9-10` |
| native_engine | externe | hors tests / cfg | crates/whisper-worker-cpu/src/native_engine.rs | `crates/whisper-worker-cpu/src/main.rs:11` |
| qualification | externe | hors tests / cfg | crates/whisper-worker-cpu/src/qualification.rs | `crates/whisper-worker-cpu/src/main.rs:12` |
| session | externe | hors tests / cfg | crates/whisper-worker-cpu/src/session/mod.rs | `crates/whisper-worker-cpu/src/main.rs:13` |
| startup | externe | hors tests / cfg | crates/whisper-worker-cpu/src/startup.rs | `crates/whisper-worker-cpu/src/main.rs:14` |
| worker | externe | hors tests / cfg | crates/whisper-worker-cpu/src/worker.rs | `crates/whisper-worker-cpu/src/main.rs:15` |
| tests | externe | tests | crates/whisper-worker-cpu/src/session/control/tests.rs | `crates/whisper-worker-cpu/src/session/control.rs:43-44` |
| control | externe | hors tests / cfg | crates/whisper-worker-cpu/src/session/control.rs | `crates/whisper-worker-cpu/src/session/mod.rs:28` |
| decode | externe | hors tests / cfg | crates/whisper-worker-cpu/src/session/decode.rs | `crates/whisper-worker-cpu/src/session/mod.rs:29` |
| import | externe | hors tests / cfg | crates/whisper-worker-cpu/src/session/import.rs | `crates/whisper-worker-cpu/src/session/mod.rs:30` |
| live | externe | hors tests / cfg | crates/whisper-worker-cpu/src/session/live.rs | `crates/whisper-worker-cpu/src/session/mod.rs:31` |
| tests | externe | tests | crates/whisper-worker-cpu/src/worker/tests.rs | `crates/whisper-worker-cpu/src/worker.rs:33-34` |
| support | externe | tests | crates/whisper-worker-cpu/tests/support/worker.rs | `crates/whisper-worker-cpu/tests/reusable_imports.rs:11-12` |
| tests | inline | tests | plage locale | `crates/whisper-worker-cpu/tests/support/worker.rs:212-282` |
| reusable_imports | externe | tests | crates/whisper-worker-cpu/tests/reusable_imports.rs | `crates/whisper-worker-cpu/tests/worker_contract.rs:15-16` |
| support | externe | tests | crates/whisper-worker-cpu/tests/support/worker.rs | `crates/whisper-worker-cpu/tests/worker_contract.rs:17-18` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate::decoder::tests — `crates/whisper-worker-cpu/src/decoder/tests.rs`

Tests ou supports : définit `inaccessible_source_propagates_error` `crates/whisper-worker-cpu/src/decoder/tests.rs:4-11`.

Mesure : 0 production / 11 tests / 11 physiques. `crates/whisper-worker-cpu/src/decoder/tests.rs:1-11`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/decoder/tests.rs:1-11`.

Fonctions avec corps : `inaccessible_source_propagates_error` [tests] `crates/whisper-worker-cpu/src/decoder/tests.rs:4-11`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/decoder/tests.rs:1-11`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/decoder/tests.rs:1-11`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::decoder — `crates/whisper-worker-cpu/src/decoder.rs`

Adapte les fonctions de décodage partagées à l’observateur local. `crates/whisper-worker-cpu/src/decoder.rs:1-29`

Mesure : 27 production / 2 tests / 29 physiques. `crates/whisper-worker-cpu/src/decoder.rs:1-29`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/decoder.rs:1-29`.

Fonctions avec corps : `observer` `crates/whisper-worker-cpu/src/decoder.rs:6-15`; `decode_requested_block` `crates/whisper-worker-cpu/src/decoder.rs:17-19`; `decode_wav_windows` `crates/whisper-worker-cpu/src/decoder.rs:21-26`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_worker_common::decoder::{MAX_WINDOW_SAMPLES, OUTPUT_RATE};` `crates/whisper-worker-cpu/src/decoder.rs:4`; `pub fn decode_requested_block(request: &DecodeRequest) -> Result<Option<Vec<i16>>, String> {` `crates/whisper-worker-cpu/src/decoder.rs:17`; `pub fn decode_wav_windows(` `crates/whisper-worker-cpu/src/decoder.rs:21`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/decoder.rs:1-29`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

### crate::diagnostics — `crates/whisper-worker-cpu/src/diagnostics.rs`

Émet les événements IPC de défaillance. `crates/whisper-worker-cpu/src/diagnostics.rs:1-24`

Mesure : 24 production / 0 tests / 24 physiques. `crates/whisper-worker-cpu/src/diagnostics.rs:1-24`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/diagnostics.rs:1-24`.

Fonctions avec corps : `emit_failure` `crates/whisper-worker-cpu/src/diagnostics.rs:5-24`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/diagnostics.rs:1-24`.

Dépend de, via `use crate::` : hors tests : `crate::ipc_output::write_frame` → `crate::ipc_output` `crates/whisper-worker-cpu/src/diagnostics.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

### crate::encoder::tests — `crates/whisper-worker-cpu/src/encoder/tests.rs`

Tests ou supports : définit `capture_encoder_baseline` `crates/whisper-worker-cpu/src/encoder/tests.rs:5-34`.

Mesure : 0 production / 34 tests / 34 physiques. `crates/whisper-worker-cpu/src/encoder/tests.rs:1-34`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/encoder/tests.rs:1-34`.

Fonctions avec corps : `capture_encoder_baseline` [tests] `crates/whisper-worker-cpu/src/encoder/tests.rs:5-34`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/encoder/tests.rs:1-34`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/encoder/tests.rs:1-34`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::encoder — `crates/whisper-worker-cpu/src/encoder.rs`

Réexporte l’encodeur MP3 partagé. `crates/whisper-worker-cpu/src/encoder.rs:1-4`

Mesure : 2 production / 2 tests / 4 physiques. `crates/whisper-worker-cpu/src/encoder.rs:1-4`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/encoder.rs:1-4`.

Fonctions avec corps : aucune relevée dans `crates/whisper-worker-cpu/src/encoder.rs:1-4`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_worker_common::encoder::LiveMp3Encoder;` `crates/whisper-worker-cpu/src/encoder.rs:1`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/encoder.rs:1-4`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

### crate::engine_cache — `crates/whisper-worker-cpu/src/engine_cache.rs`

Vérifie l’identité demandée avant réutilisation ou chargement du moteur. `crates/whisper-worker-cpu/src/engine_cache.rs:1-23`

Mesure : 23 production / 0 tests / 23 physiques. `crates/whisper-worker-cpu/src/engine_cache.rs:1-23`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/engine_cache.rs:1-23`.

Fonctions avec corps : `cached_cpu_engine` `crates/whisper-worker-cpu/src/engine_cache.rs:3-23`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/engine_cache.rs:1-23`.

Dépend de, via `use crate::` : hors tests : `crate::native_engine::CpuEngine` → `crate::native_engine` `crates/whisper-worker-cpu/src/engine_cache.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

### crate::integrity — `crates/whisper-worker-cpu/src/integrity.rs`

Réexporte la vérification SHA-256 partagée. `crates/whisper-worker-cpu/src/integrity.rs:1`

Mesure : 1 production / 0 tests / 1 physiques. `crates/whisper-worker-cpu/src/integrity.rs:1`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/integrity.rs:1`.

Fonctions avec corps : aucune relevée dans `crates/whisper-worker-cpu/src/integrity.rs:1`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/integrity.rs:1`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/integrity.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

### crate::ipc::tests — `crates/whisper-worker-cpu/src/ipc/tests.rs`

Tests ou supports : définit `rejects_oversized_and_incomplete_frames_before_deserialization` `crates/whisper-worker-cpu/src/ipc/tests.rs:4-20`, `rejects_serialized_frames_over_one_mibibyte` `crates/whisper-worker-cpu/src/ipc/tests.rs:23-29`.

Mesure : 0 production / 29 tests / 29 physiques. `crates/whisper-worker-cpu/src/ipc/tests.rs:1-29`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/ipc/tests.rs:1-29`.

Fonctions avec corps : `rejects_oversized_and_incomplete_frames_before_deserialization` [tests] `crates/whisper-worker-cpu/src/ipc/tests.rs:4-20`; `rejects_serialized_frames_over_one_mibibyte` [tests] `crates/whisper-worker-cpu/src/ipc/tests.rs:23-29`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/ipc/tests.rs:1-29`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/ipc/tests.rs:1-29`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc — `crates/whisper-worker-cpu/src/ipc.rs`

Adapte les trames partagées à l’observateur local et définit Incoming. `crates/whisper-worker-cpu/src/ipc.rs:1-30`

Mesure : 26 production / 4 tests / 30 physiques. `crates/whisper-worker-cpu/src/ipc.rs:1-30`

Types : `Incoming` (type, hors tests) `crates/whisper-worker-cpu/src/ipc.rs:26-27`.

Fonctions avec corps : `observer` `crates/whisper-worker-cpu/src/ipc.rs:7-16`; `read_frame` `crates/whisper-worker-cpu/src/ipc.rs:18-20`; `write_frame` `crates/whisper-worker-cpu/src/ipc.rs:22-24`.

Déclarations `pub` (visibilité sous conditions) : `pub use whisper_worker_common::ipc::MAX_FRAME_BYTES;` `crates/whisper-worker-cpu/src/ipc.rs:5`; `pub fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead) -> io::Result<Option<T>> {` `crates/whisper-worker-cpu/src/ipc.rs:18`; `pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {` `crates/whisper-worker-cpu/src/ipc.rs:22`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/ipc.rs:1-30`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:2`<br>hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::worker` `crates/whisper-worker-cpu/src/worker.rs:1`.

### crate::ipc_output::tests — `crates/whisper-worker-cpu/src/ipc_output/tests.rs`

Tests ou supports : définit `writer_accepts_limit_and_rejects_limit_plus_one_without_growing_past_it` `crates/whisper-worker-cpu/src/ipc_output/tests.rs:4-15`.

Mesure : 0 production / 15 tests / 15 physiques. `crates/whisper-worker-cpu/src/ipc_output/tests.rs:1-15`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/ipc_output/tests.rs:1-15`.

Fonctions avec corps : `writer_accepts_limit_and_rejects_limit_plus_one_without_growing_past_it` [tests] `crates/whisper-worker-cpu/src/ipc_output/tests.rs:4-15`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/ipc_output/tests.rs:1-15`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/ipc_output/tests.rs:1-15`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc_output — `crates/whisper-worker-cpu/src/ipc_output.rs`

Écrit et vide les trames de sortie IPC. `crates/whisper-worker-cpu/src/ipc_output.rs:1-14`

Mesure : 10 production / 4 tests / 14 physiques. `crates/whisper-worker-cpu/src/ipc_output.rs:1-14`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/ipc_output.rs:1-14`.

Fonctions avec corps : `write_frame` `crates/whisper-worker-cpu/src/ipc_output.rs:6-11`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/ipc_output.rs:1-14`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/ipc_output.rs:1-14`.

Utilisé par, via ces imports : hors tests : `crate::diagnostics` `crates/whisper-worker-cpu/src/diagnostics.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:5`.

### crate — `crates/whisper-worker-cpu/src/main.rs`

Délègue le point d’entrée à startup::run. `crates/whisper-worker-cpu/src/main.rs:1-19`

Mesure : 19 production / 0 tests / 19 physiques. `crates/whisper-worker-cpu/src/main.rs:1-19`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/main.rs:1-19`.

Fonctions avec corps : `main` `crates/whisper-worker-cpu/src/main.rs:17-19`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/main.rs:1-19`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/main.rs:1-19`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::memory_qualification — `crates/whisper-worker-cpu/src/memory_qualification.rs`

Écrit les mesures mémoire conditionnelles du worker. `crates/whisper-worker-cpu/src/memory_qualification.rs:1-92`

Mesure : 92 production / 0 tests / 92 physiques. `crates/whisper-worker-cpu/src/memory_qualification.rs:1-92`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/memory_qualification.rs:1-92`.

Fonctions avec corps : `record` `crates/whisper-worker-cpu/src/memory_qualification.rs:12-62`; `process_memory_sample` `crates/whisper-worker-cpu/src/memory_qualification.rs:64-92`.

Déclarations `pub` (visibilité sous conditions) : `pub fn record(stage: &str, details: Value) {` `crates/whisper-worker-cpu/src/memory_qualification.rs:12`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/memory_qualification.rs:1-92`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:2`<br>hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:4`.

### crate::native_engine — `crates/whisper-worker-cpu/src/native_engine.rs`

Charge whisper-rs et transforme ses résultats de transcription. `crates/whisper-worker-cpu/src/native_engine.rs:1-196`

Mesure : 196 production / 0 tests / 196 physiques. `crates/whisper-worker-cpu/src/native_engine.rs:1-196`

Types : `CpuEngine` (struct, hors tests) `crates/whisper-worker-cpu/src/native_engine.rs:6-8`; `TextSegment` (struct, hors tests) `crates/whisper-worker-cpu/src/native_engine.rs:10-14`.

Fonctions avec corps : `load` `crates/whisper-worker-cpu/src/native_engine.rs:17-54`; `transcribe` `crates/whisper-worker-cpu/src/native_engine.rs:56-195`.

Déclarations `pub` (visibilité sous conditions) : `pub const MAX_LIVE_WINDOW_SAMPLES: usize = 128_000;` `crates/whisper-worker-cpu/src/native_engine.rs:4`; `pub struct CpuEngine {` `crates/whisper-worker-cpu/src/native_engine.rs:6`; `pub struct TextSegment {` `crates/whisper-worker-cpu/src/native_engine.rs:10`; `pub fn load(path: &str) -> Result<Self, String> {` `crates/whisper-worker-cpu/src/native_engine.rs:17`; `pub fn transcribe(` `crates/whisper-worker-cpu/src/native_engine.rs:56`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/native_engine.rs:1-196`.

Utilisé par, via ces imports : hors tests : `crate::engine_cache` `crates/whisper-worker-cpu/src/engine_cache.rs:1`<br>hors tests : `crate::session::import` `crates/whisper-worker-cpu/src/session/import.rs:3`<br>hors tests : `crate::session::live` `crates/whisper-worker-cpu/src/session/live.rs:2`<br>hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::worker` `crates/whisper-worker-cpu/src/worker.rs:1`.

### crate::qualification — `crates/whisper-worker-cpu/src/qualification.rs`

Implémente résultats/barrières de campagne L04 conditionnés par recette-hooks. `crates/whisper-worker-cpu/src/qualification.rs:1-307`

Mesure : 307 production / 0 tests / 307 physiques. `crates/whisper-worker-cpu/src/qualification.rs:1-307`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/qualification.rs:1-307`.

Fonctions avec corps : `l04_campaign_worker_result` `crates/whisper-worker-cpu/src/qualification.rs:8-71`; `l04_campaign_worker_result` `crates/whisper-worker-cpu/src/qualification.rs:74-82`; `l04_campaign_gate` `crates/whisper-worker-cpu/src/qualification.rs:85-297`; `l04_campaign_gate` `crates/whisper-worker-cpu/src/qualification.rs:300-307`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/qualification.rs:1-307`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/qualification.rs:1-307`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::session` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

### crate::session::control::tests — `crates/whisper-worker-cpu/src/session/control/tests.rs`

Tests ou supports : définit `empty_and_disconnected_channels_do_not_stop_the_session` `crates/whisper-worker-cpu/src/session/control/tests.rs:4-11`, `wrong_stop_is_ignored_and_matching_stop_is_consumed` `crates/whisper-worker-cpu/src/session/control/tests.rs:14-27`.

Mesure : 0 production / 47 tests / 47 physiques. `crates/whisper-worker-cpu/src/session/control/tests.rs:1-47`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/session/control/tests.rs:1-47`.

Fonctions avec corps : `empty_and_disconnected_channels_do_not_stop_the_session` [tests] `crates/whisper-worker-cpu/src/session/control/tests.rs:4-11`; `wrong_stop_is_ignored_and_matching_stop_is_consumed` [tests] `crates/whisper-worker-cpu/src/session/control/tests.rs:14-27`; `wrong_protocol_emits_correlated_failure_before_stopping` [tests] `crates/whisper-worker-cpu/src/session/control/tests.rs:30-47`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/session/control/tests.rs:1-47`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/session/control/tests.rs:1-47`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::control — `crates/whisper-worker-cpu/src/session/control.rs`

Draine les commandes d’arrêt et contrôle leur identité. `crates/whisper-worker-cpu/src/session/control.rs:1-44`

Mesure : 42 production / 2 tests / 44 physiques. `crates/whisper-worker-cpu/src/session/control.rs:1-44`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/session/control.rs:1-44`.

Fonctions avec corps : `take_stop` `crates/whisper-worker-cpu/src/session/control.rs:3-41`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/session/control.rs:1-44`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/session/control.rs:1-44`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::decode — `crates/whisper-worker-cpu/src/session/decode.rs`

Traite DecodeBlock et émet les blocs PCM. `crates/whisper-worker-cpu/src/session/decode.rs:1-148`

Mesure : 148 production / 0 tests / 148 physiques. `crates/whisper-worker-cpu/src/session/decode.rs:1-148`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/session/decode.rs:1-148`.

Fonctions avec corps : `handle_decode` `crates/whisper-worker-cpu/src/session/decode.rs:3-91`; `run_decode_service` `crates/whisper-worker-cpu/src/session/decode.rs:93-148`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/session/decode.rs:1-148`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/session/decode.rs:1-148`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::import — `crates/whisper-worker-cpu/src/session/import.rs`

Vérifie source/modèle et exécute la transcription import par fenêtres. `crates/whisper-worker-cpu/src/session/import.rs:1-293`

Mesure : 293 production / 0 tests / 293 physiques. `crates/whisper-worker-cpu/src/session/import.rs:1-293`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/session/import.rs:1-293`.

Fonctions avec corps : `run_import` `crates/whisper-worker-cpu/src/session/import.rs:4-293`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/session/import.rs:1-293`.

Dépend de, via `use crate::` : hors tests : `crate::native_engine` → `crate::native_engine` `crates/whisper-worker-cpu/src/session/import.rs:3`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session::live — `crates/whisper-worker-cpu/src/session/live.rs`

Exécute les commandes audio live et émet segments, paquets et fins de fenêtres. `crates/whisper-worker-cpu/src/session/live.rs:1-254`

Mesure : 254 production / 0 tests / 254 physiques. `crates/whisper-worker-cpu/src/session/live.rs:1-254`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/session/live.rs:1-254`.

Fonctions avec corps : `run_live_service` `crates/whisper-worker-cpu/src/session/live.rs:4-254`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/session/live.rs:1-254`.

Dépend de, via `use crate::` : hors tests : `crate::native_engine` → `crate::native_engine` `crates/whisper-worker-cpu/src/session/live.rs:2`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::session — `crates/whisper-worker-cpu/src/session/mod.rs`

Vérifie le protocole et route les commandes import/live/décodage. `crates/whisper-worker-cpu/src/session/mod.rs:1-169`

Mesure : 169 production / 0 tests / 169 physiques. `crates/whisper-worker-cpu/src/session/mod.rs:1-169`

Types : `ImportRequest` (struct, hors tests) `crates/whisper-worker-cpu/src/session/mod.rs:35-45`.

Fonctions avec corps : `run_session` `crates/whisper-worker-cpu/src/session/mod.rs:47-169`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/session/mod.rs:1-169`.

Dépend de, via `use crate::` : hors tests : `crate::memory_qualification` → `crate::memory_qualification` `crates/whisper-worker-cpu/src/session/mod.rs:2`<br>hors tests : `crate::decoder` → `crate::decoder` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::diagnostics::emit_failure` → `crate::diagnostics` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::encoder` → `crate::encoder` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::engine_cache::cached_cpu_engine` → `crate::engine_cache` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::integrity::verify_sha256` → `crate::integrity` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::ipc::Incoming` → `crate::ipc` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::ipc_output::write_frame` → `crate::ipc_output` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::native_engine::CpuEngine` → `crate::native_engine` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::qualification::l04_campaign_gate` → `crate::qualification` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::qualification::l04_campaign_worker_result` → `crate::qualification` `crates/whisper-worker-cpu/src/session/mod.rs:3`<br>hors tests : `crate::qualification` → `crate::qualification` `crates/whisper-worker-cpu/src/session/mod.rs:3`.

Utilisé par, via ces imports : hors tests : `crate::worker` `crates/whisper-worker-cpu/src/worker.rs:1`.

### crate::startup — `crates/whisper-worker-cpu/src/startup.rs`

Ouvre stdin/stdout, crée le canal et lance le thread lecteur IPC. `crates/whisper-worker-cpu/src/startup.rs:1-86`

Mesure : 86 production / 0 tests / 86 physiques. `crates/whisper-worker-cpu/src/startup.rs:1-86`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/startup.rs:1-86`.

Fonctions avec corps : `run` `crates/whisper-worker-cpu/src/startup.rs:17-43`; `input_loop` `crates/whisper-worker-cpu/src/startup.rs:45-86`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/startup.rs:1-86`.

Dépend de, via `use crate::` : hors tests : `crate::ipc` → `crate::ipc` `crates/whisper-worker-cpu/src/startup.rs:2`<br>hors tests : `crate::memory_qualification` → `crate::memory_qualification` `crates/whisper-worker-cpu/src/startup.rs:4`<br>hors tests : `crate::ipc::Incoming` → `crate::ipc` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::ipc::read_frame` → `crate::ipc` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::ipc` → `crate::ipc` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::ipc_output::write_frame` → `crate::ipc_output` `crates/whisper-worker-cpu/src/startup.rs:5`<br>hors tests : `crate::worker::run` → `crate::worker` `crates/whisper-worker-cpu/src/startup.rs:5`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker::tests — `crates/whisper-worker-cpu/src/worker/tests.rs`

Tests ou supports : définit `idle_worker_shutdown_exits_without_an_error_frame` `crates/whisper-worker-cpu/src/worker/tests.rs:6-14`.

Mesure : 0 production / 14 tests / 14 physiques. `crates/whisper-worker-cpu/src/worker/tests.rs:1-14`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/worker/tests.rs:1-14`.

Fonctions avec corps : `idle_worker_shutdown_exits_without_an_error_frame` [tests] `crates/whisper-worker-cpu/src/worker/tests.rs:6-14`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/worker/tests.rs:1-14`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/src/worker/tests.rs:1-14`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::worker — `crates/whisper-worker-cpu/src/worker.rs`

Boucle sur les sessions et conserve le cache moteur entre demandes. `crates/whisper-worker-cpu/src/worker.rs:1-34`

Mesure : 32 production / 2 tests / 34 physiques. `crates/whisper-worker-cpu/src/worker.rs:1-34`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/src/worker.rs:1-34`.

Fonctions avec corps : `run` `crates/whisper-worker-cpu/src/worker.rs:5-31`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/src/worker.rs:1-34`.

Dépend de, via `use crate::` : hors tests : `crate::ipc::Incoming` → `crate::ipc` `crates/whisper-worker-cpu/src/worker.rs:1`<br>hors tests : `crate::native_engine::CpuEngine` → `crate::native_engine` `crates/whisper-worker-cpu/src/worker.rs:1`<br>hors tests : `crate::session::run_session` → `crate::session` `crates/whisper-worker-cpu/src/worker.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::startup` `crates/whisper-worker-cpu/src/startup.rs:5`.

### integration::live_worker — `crates/whisper-worker-cpu/tests/live_worker.rs`

Tests ou supports : définit `live_worker_rejects_an_unverified_model_before_reporting_ready` `crates/whisper-worker-cpu/tests/live_worker.rs:11-44`.

Mesure : 0 production / 44 tests / 44 physiques. `crates/whisper-worker-cpu/tests/live_worker.rs:1-44`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/tests/live_worker.rs:1-44`.

Fonctions avec corps : `live_worker_rejects_an_unverified_model_before_reporting_ready` [tests] `crates/whisper-worker-cpu/tests/live_worker.rs:11-44`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/tests/live_worker.rs:1-44`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/tests/live_worker.rs:1-44`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::reusable_imports / integration::worker_contract::reusable_imports — `crates/whisper-worker-cpu/tests/reusable_imports.rs`

Tests ou supports : définit `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache` `crates/whisper-worker-cpu/tests/reusable_imports.rs:114-116`.

Mesure : 0 production / 220 tests / 220 physiques. `crates/whisper-worker-cpu/tests/reusable_imports.rs:1-220`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/tests/reusable_imports.rs:1-220`.

Fonctions avec corps : `hash_file` [tests] `crates/whisper-worker-cpu/tests/reusable_imports.rs:20-32`; `write_silent_wav` [tests] `crates/whisper-worker-cpu/tests/reusable_imports.rs:34-52`; `send` [tests] `crates/whisper-worker-cpu/tests/reusable_imports.rs:54-58`; `await_end` [tests] `crates/whisper-worker-cpu/tests/reusable_imports.rs:60-110`; `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache` [tests] `crates/whisper-worker-cpu/tests/reusable_imports.rs:114-116`; `run_for_exe` [tests] `crates/whisper-worker-cpu/tests/reusable_imports.rs:118-220`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/tests/reusable_imports.rs:1-220`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/tests/reusable_imports.rs:1-220`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::reusable_imports::support / integration::support::worker / integration::worker_contract::reusable_imports::support / integration::worker_contract::support — `crates/whisper-worker-cpu/tests/support/worker.rs`

Tests ou supports : définit `deadline_child_fixture` `crates/whisper-worker-cpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-cpu/tests/support/worker.rs:226-259`.

Mesure : 0 production / 282 tests / 282 physiques. `crates/whisper-worker-cpu/tests/support/worker.rs:1-282`

Types : `BoundedChild` (struct, tests) `crates/whisper-worker-cpu/tests/support/worker.rs:16-22`.

Fonctions avec corps : `spawn` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:25-50`; `id` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:52-54`; `stdin` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:55-57`; `stdout` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:58-60`; `stderr` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:61-63`; `remaining` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:64-66`; `timed_out` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:67-69`; `wait` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:71-106`; `drop` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:110-131`; `finish_reader` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:134-143`; `output` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:145-210`; `deadline_child_fixture` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:217-223`; `deadline_stops_child_after_expected_output` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:226-259`; `timed_out_output_retains_the_captured_stdout` [tests] `crates/whisper-worker-cpu/tests/support/worker.rs:262-281`.

Déclarations `pub` (visibilité sous conditions) : `pub const CONTROLLED_TIMEOUT: Duration = Duration::from_secs(300);` `crates/whisper-worker-cpu/tests/support/worker.rs:13`; `pub const NATIVE_TIMEOUT: Duration = Duration::from_secs(600);` `crates/whisper-worker-cpu/tests/support/worker.rs:14`; `pub struct BoundedChild {` `crates/whisper-worker-cpu/tests/support/worker.rs:16`; `pub fn spawn(command: &mut Command, timeout: Duration) -> Self {` `crates/whisper-worker-cpu/tests/support/worker.rs:25`; `pub fn id(&self) -> u32 {` `crates/whisper-worker-cpu/tests/support/worker.rs:52`; `pub fn stdin(&self) -> ChildStdin {` `crates/whisper-worker-cpu/tests/support/worker.rs:55`; `pub fn stdout(&self) -> ChildStdout {` `crates/whisper-worker-cpu/tests/support/worker.rs:58`; `pub fn stderr(&self) -> ChildStderr {` `crates/whisper-worker-cpu/tests/support/worker.rs:61`; `pub fn remaining(&self) -> Duration {` `crates/whisper-worker-cpu/tests/support/worker.rs:64`; `pub fn timed_out(&self) -> bool {` `crates/whisper-worker-cpu/tests/support/worker.rs:67`; `pub fn wait(&self) -> Result<ExitStatus, String> {` `crates/whisper-worker-cpu/tests/support/worker.rs:71`; `pub fn finish_reader<T>(reader: JoinHandle<T>, child: &BoundedChild) -> T {` `crates/whisper-worker-cpu/tests/support/worker.rs:134`; `pub fn output(command: &mut Command, input: Option<Vec<u8>>, timeout: Duration) -> Output {` `crates/whisper-worker-cpu/tests/support/worker.rs:145`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/tests/support/worker.rs:1-282`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `deadline_child_fixture` `crates/whisper-worker-cpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-cpu/tests/support/worker.rs:226-259`, `timed_out_output_retains_the_captured_stdout` `crates/whisper-worker-cpu/tests/support/worker.rs:262-281`. `crates/whisper-worker-cpu/tests/support/worker.rs:212-282`

Mesure locale : 0 production / 71 tests ; sous-ensemble non additionnable. `crates/whisper-worker-cpu/tests/support/worker.rs:212-282`

Types locaux : aucun relevé dans `crates/whisper-worker-cpu/tests/support/worker.rs:212-282`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-cpu/tests/support/worker.rs:212-282`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### integration::worker_contract — `crates/whisper-worker-cpu/tests/worker_contract.rs`

Tests ou supports : définit `turbo_idle_and_refusal_contract` `crates/whisper-worker-cpu/tests/worker_contract.rs:31-65`, `turbo_reusable_import_contract` `crates/whisper-worker-cpu/tests/worker_contract.rs:69-78`.

Mesure : 0 production / 246 tests / 246 physiques. `crates/whisper-worker-cpu/tests/worker_contract.rs:1-246`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-cpu/tests/worker_contract.rs:1-246`.

Fonctions avec corps : `run_command` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:20-28`; `turbo_idle_and_refusal_contract` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:31-65`; `turbo_reusable_import_contract` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:69-78`; `model_hash` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:80-92`; `send_command` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:94-98`; `live_case` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:100-228`; `turbo_live_stop_and_shutdown_contract` [tests] `crates/whisper-worker-cpu/tests/worker_contract.rs:232-246`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-cpu/tests/worker_contract.rs:1-246`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-cpu/tests/worker_contract.rs:1-246`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 0 | 2 |
| cfg_feature | 32 | 0 |
| cfg_test | 0 | 9 |
| command_new | 1 | 6 |
| expect | 1 | 18 |
| thread_spawn | 1 | 7 |
| unsafe | 0 | 0 |
| unwrap | 0 | 58 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 18 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 10 sur plages hors tests et 8 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::duplicate_mod | 0 | 1 |
| clippy::ok_expect | 0 | 2 |
| clippy::too_many_arguments | 2 | 0 |
| clippy::too_many_lines | 8 | 1 |
| dead_code | 0 | 4 |
