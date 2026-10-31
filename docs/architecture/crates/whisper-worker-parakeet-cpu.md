Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-worker-parakeet-cpu

## Rôle et type

Charge Parakeet depuis ses bibliothèques natives puis traite son protocole IPC. `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71` ; `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233` ; `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410` ; `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174` ; `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`

Le backend dépend de la feature et le mode décodage seul est distinct de la transcription. `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71` ; `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233` ; `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410` ; `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174` ; `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`

Type : **Worker binaire Parakeet, backend conditionnel**. `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71` ; `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233` ; `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410` ; `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174` ; `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`

Mesures : **1877 production / 3407 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-worker-parakeet-cpu/Cargo.toml:15` |
| serde | externe | dependencies | {"workspace": true} | `crates/whisper-worker-parakeet-cpu/Cargo.toml:16` |
| serde_json | externe | dependencies | {"workspace": true} | `crates/whisper-worker-parakeet-cpu/Cargo.toml:17` |
| sha2 | externe | dependencies | {"workspace": true} | `crates/whisper-worker-parakeet-cpu/Cargo.toml:18` |
| symphonia | externe | dependencies | {"workspace": true} | `crates/whisper-worker-parakeet-cpu/Cargo.toml:19` |
| libloading | externe | dependencies | {"workspace": true} | `crates/whisper-worker-parakeet-cpu/Cargo.toml:20` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-worker-parakeet-cpu/Cargo.toml:1-26`.

## Interfaces publiques et usages intercrates

[Non vérifié] Aucun import consommateur d’une autre crate relevé ; les autres usages ne sont pas exclus.

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21`. Imports textuels, pas un graphe de symboles résolu.

Interface entre processus : `crates/whisper-core/src/parakeet_ipc.rs:1-130`.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:30` ; `crates/whisper-worker-parakeet-cpu/src/decoder.rs:1` ; `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:64` ; `crates/whisper-worker-parakeet-cpu/src/integrity.rs:2` ; `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:6` |
| Processus | `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:133` |
| Threads/canaux | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Entrées/sorties standard | `crates/whisper-worker-parakeet-cpu/src/ipc.rs:36` ; `crates/whisper-worker-parakeet-cpu/src/session.rs:19` |
| GPU/natif | `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:17` |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate::args | Analyse les arguments de lancement du worker. `crates/whisper-worker-parakeet-cpu/src/args.rs:1-78` | 78 | 0 |
| crate::decode_only | Exécute le décodage seul vers un fichier WAV. `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49` | 49 | 0 |
| crate::decoder | Décode les fichiers audio avec Symphonia en PCM Parakeet. `crates/whisper-worker-parakeet-cpu/src/decoder.rs:1-149` | 149 | 0 |
| crate::gpu_identity::tests | Tests ou supports : définit `nvidia_inventory_parser_requires_exactly_one_consistent_gpu_row` `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:5-24`. | 0 | 24 |
| crate::gpu_identity | Détermine le backend et vérifie les preuves GPU avec nvidia-smi. `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174` | 172 | 2 |
| crate::integrity | Empreinte runtime/modèle et compare les sommes attendues. `crates/whisper-worker-parakeet-cpu/src/integrity.rs:1-46` | 46 | 0 |
| crate::ipc::tests | Tests ou supports : définit `oversized_and_truncated_frames_are_rejected_before_json_handling` `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:5-18`. | 0 | 18 |
| crate::ipc | Lit les commandes bornées et sérialise les événements Parakeet. `crates/whisper-worker-parakeet-cpu/src/ipc.rs:1-64` | 62 | 2 |
| crate | Délègue le point d’entrée à startup::run. `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21` | 21 | 0 |
| crate::native_engine | Enveloppe le moteur natif et normalise mots/horodatages. `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:1-575` | 318 | 257 |
| crate::native_ffi | Charge les DLL/symboles Parakeet et gère pointeurs/résultats natifs. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410` | 376 | 34 |
| crate::reference_decoder::tests | Tests ou supports : définit `prepare_reference_inputs_for_campaign` `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:10-54`, `reference_wav_decoder_matches_product_decoder_and_reads_sixty_minutes` `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:58-70`. | 0 | 106 |
| crate::reference_decoder | Décode le WAV de référence après contrôle du format. `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1-65` | 63 | 2 |
| crate::runtime_paths::personal_tests | Tests ou supports : définit `personal_worker_accepts_only_published_root_or_a_direct_staging_child` `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:4-19`. | 0 | 19 |
| crate::runtime_paths::tests | Tests ou supports : définit `accepts_only_the_two_canonical_l2_test_roots` `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:16-50`, `gpu_worker_allows_only_the_canonical_l3_runtime_roots` `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:54-77`. | 0 | 106 |
| crate::runtime_paths | Vérifie les racines autorisées des ressources et du profil personnel. `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:1-157` | 153 | 4 |
| crate::session | Lit les commandes Parakeet et émet disponibilité, résultats et échecs corrélés. `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233` | 233 | 0 |
| crate::startup | Choisit le mode, valide ressources/empreintes et charge le moteur natif. `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71` | 71 | 0 |
| crate::transcription::tests | Tests ou supports : définit `native_error_classes_preserve_the_original_message` `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:4-15`. | 0 | 15 |
| crate::transcription | Vérifie la demande unitaire et appelle le moteur selon son profil. `crates/whisper-worker-parakeet-cpu/src/transcription.rs:1-88` | 86 | 2 |
| integration::decoder_parity | Tests ou supports : définit `worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit` `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:20-116`. | 0 | 224 |
| integration::native_diagnostics_absence | Tests ou supports : définit `native_workers_preserve_transcription_without_historical_diagnostics` `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:325-348`. | 0 | 348 |
| integration::native_import | Tests ou supports : définit `additive_gpu_fields_default_when_reading_existing_cpu_ipc_frames` `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:30-63`, `cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload` `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:67-355`. | 0 | 1521 |
| integration::native_diagnostics_absence::support / integration::support::worker / integration::worker_contract::support | Tests ou supports : définit `deadline_child_fixture` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:226-259`. | 0 | 282 |
| integration::worker_contract | Tests ou supports : définit `parakeet_decode_only_contract` `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:63-83`, `parakeet_refusal_order_contract` `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:352-355`. | 0 | 441 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| tests | externe | tests | crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs | `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:173-174` |
| tests | externe | tests | crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs | `crates/whisper-worker-parakeet-cpu/src/ipc.rs:63-64` |
| args | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/args.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:1` |
| decode_only | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/decode_only.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:2` |
| decoder | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/decoder.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:3` |
| gpu_identity | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:4` |
| integrity | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/integrity.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:5` |
| ipc | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/ipc.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:6` |
| native_engine | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/native_engine.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:7` |
| native_ffi | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/native_ffi.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:8` |
| reference_decoder | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:9-10` |
| runtime_paths | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:11` |
| session | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/session.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:12` |
| startup | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/startup.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:13` |
| transcription | externe | hors tests / cfg | crates/whisper-worker-parakeet-cpu/src/transcription.rs | `crates/whisper-worker-parakeet-cpu/src/main.rs:14` |
| tests | inline | tests | plage locale | `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:319-575` |
| backend_selection_tests | inline | tests | plage locale | `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:351-361` |
| tests | inline | tests | plage locale | `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:388-410` |
| tests | externe | tests | crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs | `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:64-65` |
| personal_tests | externe | tests | crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs | `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:154-155` |
| tests | externe | tests | crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs | `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:156-157` |
| tests | externe | tests | crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs | `crates/whisper-worker-parakeet-cpu/src/transcription.rs:11-12` |
| support | externe | tests | crates/whisper-worker-parakeet-cpu/tests/support/worker.rs | `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:16-17` |
| tests | inline | tests | plage locale | `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:212-282` |
| support | externe | tests | crates/whisper-worker-parakeet-cpu/tests/support/worker.rs | `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:12-13` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate::args — `crates/whisper-worker-parakeet-cpu/src/args.rs`

Analyse les arguments de lancement du worker. `crates/whisper-worker-parakeet-cpu/src/args.rs:1-78`

Mesure : 78 production / 0 tests / 78 physiques. `crates/whisper-worker-parakeet-cpu/src/args.rs:1-78`

Types : `Args` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/args.rs:73-78`.

Fonctions avec corps : `parse_args` `crates/whisper-worker-parakeet-cpu/src/args.rs:5-71`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/args.rs:1-78`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/args.rs:1-78`.

Utilisé par, via ces imports : hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

### crate::decode_only — `crates/whisper-worker-parakeet-cpu/src/decode_only.rs`

Exécute le décodage seul vers un fichier WAV. `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`

Mesure : 49 production / 0 tests / 49 physiques. `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`.

Fonctions avec corps : `decode_only_mode` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:9-49`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1-49`.

Dépend de, via `use crate::` : hors tests : `crate::decoder` → `crate::decoder` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::integrity::sha256_file` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::runtime_paths::ensure_inside` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::runtime_paths::required_runtime_root` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::runtime_paths` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

### crate::decoder — `crates/whisper-worker-parakeet-cpu/src/decoder.rs`

Décode les fichiers audio avec Symphonia en PCM Parakeet. `crates/whisper-worker-parakeet-cpu/src/decoder.rs:1-149`

Mesure : 149 production / 0 tests / 149 physiques. `crates/whisper-worker-parakeet-cpu/src/decoder.rs:1-149`

Types : `DecodedAudio` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/decoder.rs:17-23`.

Fonctions avec corps : `decode_file` `crates/whisper-worker-parakeet-cpu/src/decoder.rs:25-144`; `_audio_buffer_ref_is_supported` `crates/whisper-worker-parakeet-cpu/src/decoder.rs:147-149`.

Déclarations `pub` (visibilité sous conditions) : `pub const OUTPUT_RATE: u32 = 16_000;` `crates/whisper-worker-parakeet-cpu/src/decoder.rs:13`; `pub const MAX_AUDIO_SAMPLES: usize = 28_800_000;` `crates/whisper-worker-parakeet-cpu/src/decoder.rs:14`; `pub struct DecodedAudio {` `crates/whisper-worker-parakeet-cpu/src/decoder.rs:17`; `pub fn decode_file(path: &Path) -> Result<DecodedAudio, String> {` `crates/whisper-worker-parakeet-cpu/src/decoder.rs:25`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/decoder.rs:1-149`.

Utilisé par, via ces imports : hors tests : `crate::decode_only` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::native_engine` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:5`<br>hors tests : `crate::reference_decoder` `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1`<br>hors tests : `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`.

### crate::gpu_identity::tests — `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs`

Tests ou supports : définit `nvidia_inventory_parser_requires_exactly_one_consistent_gpu_row` `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:5-24`.

Mesure : 0 production / 24 tests / 24 physiques. `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:1-24`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:1-24`.

Fonctions avec corps : `nvidia_inventory_parser_requires_exactly_one_consistent_gpu_row` [tests] `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:5-24`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:1-24`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/gpu_identity/tests.rs:1-24`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::gpu_identity — `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs`

Détermine le backend et vérifie les preuves GPU avec nvidia-smi. `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174`

Mesure : 172 production / 2 tests / 174 physiques. `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174`.

Fonctions avec corps : `worker_gpu_identity` `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:8-23`; `configured_backend` `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:25-34`; `build_gpu_evidence` `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:36-87`; `validate_expected_gpu` `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:89-129`; `query_single_nvidia_gpu` `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:132-146`; `parse_single_nvidia_gpu` `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:149-171`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/gpu_identity.rs:1-174`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

### crate::integrity — `crates/whisper-worker-parakeet-cpu/src/integrity.rs`

Empreinte runtime/modèle et compare les sommes attendues. `crates/whisper-worker-parakeet-cpu/src/integrity.rs:1-46`

Mesure : 46 production / 0 tests / 46 physiques. `crates/whisper-worker-parakeet-cpu/src/integrity.rs:1-46`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/integrity.rs:1-46`.

Fonctions avec corps : `runtime_sha256` `crates/whisper-worker-parakeet-cpu/src/integrity.rs:4-28`; `sha256_file` `crates/whisper-worker-parakeet-cpu/src/integrity.rs:30-36`; `constant_time_eq` `crates/whisper-worker-parakeet-cpu/src/integrity.rs:38-46`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/integrity.rs:1-46`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/integrity.rs:1-46`.

Utilisé par, via ces imports : hors tests : `crate::decode_only` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`<br>hors tests : `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`<br>hors tests : `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`.

### crate::ipc::tests — `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs`

Tests ou supports : définit `oversized_and_truncated_frames_are_rejected_before_json_handling` `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:5-18`.

Mesure : 0 production / 18 tests / 18 physiques. `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:1-18`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:1-18`.

Fonctions avec corps : `oversized_and_truncated_frames_are_rejected_before_json_handling` [tests] `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:5-18`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:1-18`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/ipc/tests.rs:1-18`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc — `crates/whisper-worker-parakeet-cpu/src/ipc.rs`

Lit les commandes bornées et sérialise les événements Parakeet. `crates/whisper-worker-parakeet-cpu/src/ipc.rs:1-64`

Mesure : 62 production / 2 tests / 64 physiques. `crates/whisper-worker-parakeet-cpu/src/ipc.rs:1-64`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/ipc.rs:1-64`.

Fonctions avec corps : `read_frame` `crates/whisper-worker-parakeet-cpu/src/ipc.rs:4-27`; `write_event` `crates/whisper-worker-parakeet-cpu/src/ipc.rs:29-41`; `failed` `crates/whisper-worker-parakeet-cpu/src/ipc.rs:43-61`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/ipc.rs:1-64`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/ipc.rs:1-64`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

### crate — `crates/whisper-worker-parakeet-cpu/src/main.rs`

Délègue le point d’entrée à startup::run. `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21`

Mesure : 21 production / 0 tests / 21 physiques. `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21`.

Fonctions avec corps : `main` `crates/whisper-worker-parakeet-cpu/src/main.rs:16-21`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/main.rs:1-21`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::native_engine — `crates/whisper-worker-parakeet-cpu/src/native_engine.rs`

Enveloppe le moteur natif et normalise mots/horodatages. `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:1-575`

Mesure : 318 production / 257 tests / 575 physiques. `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:1-575`

Types : `Engine` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:12-15`; `LexicalWord` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:255-262`.

Fonctions avec corps : `load` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:18-25`; `native_version` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:27-29`; `backend` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:31-40`; `transcribe` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:42-44`; `transcribe_with_start_callback` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:46-52`; `transcribe_reference` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:55-60`; `transcribe_reference_with_start_callback` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:63-69`; `transcribe_bounded` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:71-93`; `prepare_native_call` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:96-110`; `normalize_words` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:112-252`; `overlapping_run_start` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:264-278`; `common_interval` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:280-285`; `shareable_interval` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:287-290`; `raw_overlap_error` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:292-297`; `raw_word_list` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:299-310`; `is_punctuation_only` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:312-317`; `word` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:324-330`; `final_word_can_be_bounded_by_one_tdt_frame` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:333-354`; `timestamps_reject_invalid_order_and_non_final_overrun` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:357-377`; `non_positive_raw_timestamps_are_reported_with_text_and_bounds` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:380-391`; `punctuation_is_attached_without_changing_its_predecessor_interval` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:394-419`; `zero_overlap_preserves_both_intervals` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:422-426`; `eighty_ms_overlap_moves_the_next_start_when_duration_remains_positive` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:429-433`; `eighty_ms_overlap_shrinks_previous_when_next_would_become_empty` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:436-440`; `eighty_one_ms_overlap_is_rejected` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:443-448`; `same_tdt_frame_is_shared_as_equal_integer_millisecond_parts` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:451-460`; `three_words_in_one_tdt_frame_share_remainder_in_order` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:463-480`; `shareable_suffix_is_split_without_changing_the_preceding_word` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:483-501`; `nested_words_with_an_unseparable_suffix_share_the_common_interval` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:504-521`; `real_en60_the_eye_same_frame_is_split_at_618880_ms` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:524-537`; `interval_shorter_than_word_count_is_rejected_with_raw_bounds` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:540-545`; `overlap_over_eighty_ms_reports_both_raw_intervals` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:548-553`; `native_call_marker_is_not_emitted_for_duration_rejection_and_emitted_once_when_valid` [tests] `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:556-574`.

Déclarations `pub` (visibilité sous conditions) : `pub const MAX_DURATION_MS: u64 = 1_800_000;` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:7`; `pub const MAX_REFERENCE_DURATION_MS: u64 = 3_600_000;` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:9`; `pub const TDT_FRAME_MS: u32 = 80;` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:10`; `pub struct Engine {` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:12`; `pub fn load(native_dir: &Path, model: &Path) -> Result<Self, String> {` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:18`; `pub fn native_version(&self) -> &str {` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:27`; `pub fn backend(&self) -> &'static str {` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:31`; `pub fn transcribe(&mut self, audio: &DecodedAudio) -> Result<(String, Vec<Word>, u64), String> {` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:42`; `pub fn transcribe_with_start_callback(` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:46`; `pub fn transcribe_reference(` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:55`; `pub fn transcribe_reference_with_start_callback(` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:63`.

Dépend de, via `use crate::` : hors tests : `crate::decoder::DecodedAudio` → `crate::decoder` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:5`<br>hors tests : `crate::native_ffi::NativeRecognizer` → `crate::native_ffi` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:5`<br>tests : `crate::native_ffi::NativeWord` → `crate::native_ffi` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:322`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`.

#### Inline `tests`

Tests/auxiliaires : `word` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:324-330`, `final_word_can_be_bounded_by_one_tdt_frame` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:333-354`, `timestamps_reject_invalid_order_and_non_final_overrun` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:357-377`, `non_positive_raw_timestamps_are_reported_with_text_and_bounds` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:380-391`. `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:319-575`

Mesure locale : 0 production / 257 tests ; sous-ensemble non additionnable. `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:319-575`

Types locaux : aucun relevé dans `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:319-575`.

Imports locaux : `crate::native_ffi::NativeWord` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:322`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::native_ffi — `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs`

Charge les DLL/symboles Parakeet et gère pointeurs/résultats natifs. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410`

Mesure : 376 production / 34 tests / 410 physiques. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410`

Types : `BackendConfig` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:21-24`; `ModelConfig` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:26-30`; `RecognizerConfig` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:32-43`; `RecognitionOptions` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:45-60`; `Create` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:62`; `Destroy` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:63`; `Recognize` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:64-71`; `LastError` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:72`; `Version` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:73`; `ResultWordCount` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:74`; `ResultWordText` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:75`; `ResultWordTime` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:76`; `ResultDestroy` (type, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:77`; `Api` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:79-91`; `NativeRecognizer` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:93-96`; `NativeWord` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:99-103`; `NativeResult` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:106-108`; `ResultGuard` (struct, hors tests) `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:304-307`.

Fonctions avec corps : `load` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:111-205`; `version` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:207-212`; `transcribe` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:214-279`; `selected_gpu` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:282-291`; `drop` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:294-301`; `drop` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:309-313`; `native_error` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:316-322`; `copy_c_string` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:324-336`; `copy_word_text` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:338-349`; `native_backend_uses_only_the_authorized_ordinal` [tests] `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:354-360`; `decode_utf8_fragments` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:363-386`; `utf8_character_split_between_tokens_is_decoded_after_word_assembly` [tests] `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:393-396`; `invalid_word_utf8_is_attributed_without_replacement` [tests] `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:399-403`; `literal_replacement_character_is_rejected` [tests] `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:406-409`.

Déclarations `pub` (visibilité sous conditions) : `pub struct NativeRecognizer {` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:93`; `pub struct NativeWord {` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:99`; `pub struct NativeResult {` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:106`; `pub fn load(native_dir: &Path, model: &Path) -> Result<Self, String> {` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:111`; `pub fn version(&self) -> Result<String, String> {` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:207`; `pub fn transcribe(&mut self, samples: &[f32]) -> Result<NativeResult, String> {` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:214`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:1-410`.

Utilisé par, via ces imports : hors tests : `crate::native_engine` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:5`<br>tests : `crate::native_engine` `crates/whisper-worker-parakeet-cpu/src/native_engine.rs:322`.

#### Inline `backend_selection_tests`

Tests/auxiliaires : `native_backend_uses_only_the_authorized_ordinal` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:354-360`. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:351-361`

Mesure locale : 0 production / 11 tests ; sous-ensemble non additionnable. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:351-361`

Types locaux : aucun relevé dans `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:351-361`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:351-361`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

#### Inline `tests`

Tests/auxiliaires : `utf8_character_split_between_tokens_is_decoded_after_word_assembly` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:393-396`, `invalid_word_utf8_is_attributed_without_replacement` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:399-403`, `literal_replacement_character_is_rejected` `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:406-409`. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:388-410`

Mesure locale : 0 production / 23 tests ; sous-ensemble non additionnable. `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:388-410`

Types locaux : aucun relevé dans `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:388-410`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs:388-410`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::reference_decoder::tests — `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs`

Tests ou supports : définit `prepare_reference_inputs_for_campaign` `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:10-54`, `reference_wav_decoder_matches_product_decoder_and_reads_sixty_minutes` `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:58-70`.

Mesure : 0 production / 106 tests / 106 physiques. `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:1-106`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:1-106`.

Fonctions avec corps : `prepare_reference_inputs_for_campaign` [tests] `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:10-54`; `reference_wav_decoder_matches_product_decoder_and_reads_sixty_minutes` [tests] `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:58-70`; `reference_wav_decoder_rejects_audio_over_sixty_minutes` [tests] `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:74-106`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:1-106`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:1-106`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::reference_decoder — `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs`

Décode le WAV de référence après contrôle du format. `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1-65`

Mesure : 63 production / 2 tests / 65 physiques. `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1-65`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1-65`.

Fonctions avec corps : `decode_reference_wav` `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:5-62`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1-65`.

Dépend de, via `use crate::` : hors tests : `crate::decoder` → `crate::decoder` `crates/whisper-worker-parakeet-cpu/src/reference_decoder.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:2`.

### crate::runtime_paths::personal_tests — `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs`

Tests ou supports : définit `personal_worker_accepts_only_published_root_or_a_direct_staging_child` `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:4-19`.

Mesure : 0 production / 19 tests / 19 physiques. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:1-19`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:1-19`.

Fonctions avec corps : `personal_worker_accepts_only_published_root_or_a_direct_staging_child` [tests] `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:4-19`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:1-19`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/personal_tests.rs:1-19`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::runtime_paths::tests — `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs`

Tests ou supports : définit `accepts_only_the_two_canonical_l2_test_roots` `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:16-50`, `gpu_worker_allows_only_the_canonical_l3_runtime_roots` `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:54-77`.

Mesure : 0 production / 106 tests / 106 physiques. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:1-106`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:1-106`.

Fonctions avec corps : `fixture_repository` [tests] `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:2-10`; `accepts_only_the_two_canonical_l2_test_roots` [tests] `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:16-50`; `gpu_worker_allows_only_the_canonical_l3_runtime_roots` [tests] `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:54-77`; `qualification_profiles_are_confined_to_l2b` [tests] `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:81-106`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:1-106`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:1-106`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::runtime_paths — `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs`

Vérifie les racines autorisées des ressources et du profil personnel. `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:1-157`

Mesure : 153 production / 4 tests / 157 physiques. `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:1-157`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:1-157`.

Fonctions avec corps : `shared_model_runtime_allowed` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:7-29`; `required_runtime_root` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:31-68`; `personal_runtime_root_is_allowed` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:71-80`; `personal_runtime_path_is_allowed` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:83-87`; `qualification_root_is_valid` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:89-99`; `allowed_runtime_root_candidates` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:101-124`; `allowed_runtime_roots` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:126-131`; `allowed_runtime_roots_for` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:133-145`; `ensure_inside` `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:147-152`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:1-157`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:1-157`.

Utilisé par, via ces imports : hors tests : `crate::decode_only` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::decode_only` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::decode_only` `crates/whisper-worker-parakeet-cpu/src/decode_only.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

### crate::session — `crates/whisper-worker-parakeet-cpu/src/session.rs`

Lit les commandes Parakeet et émet disponibilité, résultats et échecs corrélés. `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233`

Mesure : 233 production / 0 tests / 233 physiques. `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233`.

Fonctions avec corps : `run` `crates/whisper-worker-parakeet-cpu/src/session.rs:13-233`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/session.rs:1-233`.

Dépend de, via `use crate::` : hors tests : `crate::gpu_identity::build_gpu_evidence` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::gpu_identity::validate_expected_gpu` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::gpu_identity` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::integrity::constant_time_eq` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::ipc::failed` → `crate::ipc` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::ipc::read_frame` → `crate::ipc` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::ipc::write_event` → `crate::ipc` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::ipc` → `crate::ipc` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::native_engine` → `crate::native_engine` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`<br>hors tests : `crate::transcription::transcribe_one` → `crate::transcription` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::startup` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

### crate::startup — `crates/whisper-worker-parakeet-cpu/src/startup.rs`

Choisit le mode, valide ressources/empreintes et charge le moteur natif. `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71`

Mesure : 71 production / 0 tests / 71 physiques. `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71`.

Fonctions avec corps : `run` `crates/whisper-worker-parakeet-cpu/src/startup.rs:14-71`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/startup.rs:1-71`.

Dépend de, via `use crate::` : hors tests : `crate::args::parse_args` → `crate::args` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::decode_only::decode_only_mode` → `crate::decode_only` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::gpu_identity::build_gpu_evidence` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::gpu_identity::configured_backend` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::gpu_identity::worker_gpu_identity` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::gpu_identity` → `crate::gpu_identity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::integrity::constant_time_eq` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::integrity::runtime_sha256` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::integrity::sha256_file` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::integrity` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::ipc::write_event` → `crate::ipc` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::native_engine` → `crate::native_engine` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::runtime_paths::ensure_inside` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::runtime_paths::required_runtime_root` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::runtime_paths::shared_model_runtime_allowed` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::runtime_paths` → `crate::runtime_paths` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`<br>hors tests : `crate::session` → `crate::session` `crates/whisper-worker-parakeet-cpu/src/startup.rs:1`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::transcription::tests — `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs`

Tests ou supports : définit `native_error_classes_preserve_the_original_message` `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:4-15`.

Mesure : 0 production / 15 tests / 15 physiques. `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:1-15`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:1-15`.

Fonctions avec corps : `native_error_classes_preserve_the_original_message` [tests] `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:4-15`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:1-15`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs:1-15`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::transcription — `crates/whisper-worker-parakeet-cpu/src/transcription.rs`

Vérifie la demande unitaire et appelle le moteur selon son profil. `crates/whisper-worker-parakeet-cpu/src/transcription.rs:1-88`

Mesure : 86 production / 2 tests / 88 physiques. `crates/whisper-worker-parakeet-cpu/src/transcription.rs:1-88`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-parakeet-cpu/src/transcription.rs:1-88`.

Fonctions avec corps : `classify_native_error` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:14-21`; `transcribe_one` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:23-88`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/src/transcription.rs:1-88`.

Dépend de, via `use crate::` : hors tests : `crate::reference_decoder::decode_reference_wav` → `crate::reference_decoder` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:2`<br>hors tests : `crate::decoder` → `crate::decoder` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`<br>hors tests : `crate::integrity::constant_time_eq` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`<br>hors tests : `crate::integrity::sha256_file` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`<br>hors tests : `crate::integrity` → `crate::integrity` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`<br>hors tests : `crate::native_engine` → `crate::native_engine` `crates/whisper-worker-parakeet-cpu/src/transcription.rs:3`.

Utilisé par, via ces imports : hors tests : `crate::session` `crates/whisper-worker-parakeet-cpu/src/session.rs:1`.

### integration::decoder_parity — `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs`

Tests ou supports : définit `worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit` `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:20-116`.

Mesure : 0 production / 224 tests / 224 physiques. `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:1-224`

Types : `TurboDecoder` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:160-164`.

Fonctions avec corps : `worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:20-116`; `decode_with_turbo` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:118-158`; `start` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:166-179`; `send` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:180-192`; `decode` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:193-198`; `shutdown` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:199-207`; `wait` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:208-210`; `required_path` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:213-218`; `sha256` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:219-221`; `sha256_bytes` [tests] `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:222-224`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:1-224`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs:1-224`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::native_diagnostics_absence — `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs`

Tests ou supports : définit `native_workers_preserve_transcription_without_historical_diagnostics` `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:325-348`.

Mesure : 0 production / 348 tests / 348 physiques. `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:1-348`

Types : `Outcome` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:35-39`; `Worker` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:41-46`.

Fonctions avec corps : `event` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:49-57`; `send` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:58-62`; `finish` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:63-69`; `sha` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:72-77`; `fixture` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:79-105`; `evidence_check` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:107-125`; `run` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:127-321`; `native_workers_preserve_transcription_without_historical_diagnostics` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:325-348`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:1-348`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs:1-348`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::native_import — `crates/whisper-worker-parakeet-cpu/tests/native_import.rs`

Tests ou supports : définit `additive_gpu_fields_default_when_reading_existing_cpu_ipc_frames` `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:30-63`, `cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload` `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:67-355`.

Mesure : 0 production / 1521 tests / 1521 physiques. `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1-1521`

Types : `WorkerProcess` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:665-671`; `GpuSample` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:674-679`; `ProcessMemorySample` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:682-686`; `ProcessMemorySampler` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:688-694`; `GpuCall` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:775-780`.

Fonctions avec corps : `additive_gpu_fields_default_when_reading_existing_cpu_ipc_frames` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:30-63`; `cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:67-355`; `r7_private_worker_timing` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:359-405`; `r7_private_worker_timing_child` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:409-470`; `cpu_worker_normalizes_fr87_fr27_and_en15_short_fixtures` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:474-583`; `r6_reference_en60` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:587-663`; `start` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:697-746`; `stop` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:748-761`; `drop` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:765-772`; `gpu_worker_fr_en_single_pass` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:784-983`; `gpu_worker_twenty_calls` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:987-1076`; `gpu_worker_rejects_wrong_device_before_audio` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1080-1146`; `nvidia_smi_gpu` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1148-1179`; `assert_gpu_evidence` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1181-1192`; `nvidia_smi_activity` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1194-1217`; `start_gpu_sampler` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1219-1239`; `stop_gpu_sampler` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1241-1254`; `cuda_dll_hashes` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1256-1271`; `start` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1274-1303`; `start_gpu` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1305-1342`; `start_reference` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1344-1375`; `send_raw` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1377-1381`; `request` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1383-1401`; `request_with_hash` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1403-1426`; `request_gpu` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1428-1470`; `receive` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1472-1482`; `shutdown` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1484-1487`; `wait` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1489-1491`; `json_frame` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1494-1498`; `required_path` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1500-1505`; `ensure_under` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1507-1514`; `sha256` [tests] `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1516-1521`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1-1521`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1-1521`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::native_diagnostics_absence::support / integration::support::worker / integration::worker_contract::support — `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs`

Tests ou supports : définit `deadline_child_fixture` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:226-259`.

Mesure : 0 production / 282 tests / 282 physiques. `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:1-282`

Types : `BoundedChild` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:16-22`.

Fonctions avec corps : `spawn` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:25-50`; `id` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:52-54`; `stdin` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:55-57`; `stdout` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:58-60`; `stderr` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:61-63`; `remaining` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:64-66`; `timed_out` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:67-69`; `wait` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:71-106`; `drop` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:110-131`; `finish_reader` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:134-143`; `output` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:145-210`; `deadline_child_fixture` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:217-223`; `deadline_stops_child_after_expected_output` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:226-259`; `timed_out_output_retains_the_captured_stdout` [tests] `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:262-281`.

Déclarations `pub` (visibilité sous conditions) : `pub const CONTROLLED_TIMEOUT: Duration = Duration::from_secs(300);` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:13`; `pub const NATIVE_TIMEOUT: Duration = Duration::from_secs(600);` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:14`; `pub struct BoundedChild {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:16`; `pub fn spawn(command: &mut Command, timeout: Duration) -> Self {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:25`; `pub fn id(&self) -> u32 {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:52`; `pub fn stdin(&self) -> ChildStdin {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:55`; `pub fn stdout(&self) -> ChildStdout {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:58`; `pub fn stderr(&self) -> ChildStderr {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:61`; `pub fn remaining(&self) -> Duration {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:64`; `pub fn timed_out(&self) -> bool {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:67`; `pub fn wait(&self) -> Result<ExitStatus, String> {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:71`; `pub fn finish_reader<T>(reader: JoinHandle<T>, child: &BoundedChild) -> T {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:134`; `pub fn output(command: &mut Command, input: Option<Vec<u8>>, timeout: Duration) -> Output {` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:145`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:1-282`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `deadline_child_fixture` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:217-223`, `deadline_stops_child_after_expected_output` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:226-259`, `timed_out_output_retains_the_captured_stdout` `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:262-281`. `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:212-282`

Mesure locale : 0 production / 71 tests ; sous-ensemble non additionnable. `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:212-282`

Types locaux : aucun relevé dans `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:212-282`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:212-282`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### integration::worker_contract — `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs`

Tests ou supports : définit `parakeet_decode_only_contract` `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:63-83`, `parakeet_refusal_order_contract` `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:352-355`.

Mesure : 0 production / 441 tests / 441 physiques. `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:1-441`

Types : `NativeWorker` (struct, tests) `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:85-90`.

Fonctions avec corps : `campaign_root` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:15-25`; `decode_case` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:27-59`; `parakeet_decode_only_contract` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:63-83`; `sha256` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:92-104`; `start_worker` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:106-190`; `send` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:193-198`; `event` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:199-207`; `finish` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:208-217`; `worker_paths` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:220-229`; `ready` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:231-251`; `normalize` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:253-273`; `request` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:275-286`; `refusal_case` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:288-347`; `parakeet_refusal_order_contract` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:352-355`; `errors_case` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:357-388`; `parakeet_errors_and_shutdown_contract` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:393-396`; `native_case` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:398-433`; `parakeet_native_transcription_contract` [tests] `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:438-441`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:1-441`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-parakeet-cpu/tests/worker_contract.rs:1-441`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 0 | 7 |
| cfg_feature | 41 | 15 |
| cfg_test | 0 | 8 |
| command_new | 1 | 16 |
| expect | 1 | 37 |
| thread_spawn | 0 | 7 |
| unsafe | 23 | 0 |
| unwrap | 6 | 224 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 48 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 21 sur plages hors tests et 27 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::cognitive_complexity | 0 | 2 |
| clippy::into_iter_on_ref | 2 | 0 |
| clippy::manual_contains | 2 | 0 |
| clippy::ok_expect | 0 | 2 |
| clippy::too_many_arguments | 0 | 1 |
| clippy::too_many_lines | 6 | 4 |
| clippy::type_complexity | 2 | 0 |
| clippy::while_let_loop | 2 | 0 |
| dead_code | 3 | 18 |
| unused_imports | 2 | 0 |
| unused_mut | 2 | 0 |
