Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-worker-common

## Rôle et type

Fournit décodage audio, MP3, trames JSON et vérification SHA-256. `crates/whisper-worker-common/src/decoder.rs:1-438` ; `crates/whisper-worker-common/src/encoder.rs:1-61` ; `crates/whisper-worker-common/src/ipc.rs:1-111` ; `crates/whisper-worker-common/src/integrity.rs:1-22` ; `crates/whisper-worker-common/src/observation.rs:1-4`

Expose l’alias MemoryObserver, un pointeur de fonction optionnel d’observation. `crates/whisper-worker-common/src/decoder.rs:1-438` ; `crates/whisper-worker-common/src/encoder.rs:1-61` ; `crates/whisper-worker-common/src/ipc.rs:1-111` ; `crates/whisper-worker-common/src/integrity.rs:1-22` ; `crates/whisper-worker-common/src/observation.rs:1-4`

Type : **Bibliothèque de support des workers**. `crates/whisper-worker-common/src/decoder.rs:1-438` ; `crates/whisper-worker-common/src/encoder.rs:1-61` ; `crates/whisper-worker-common/src/ipc.rs:1-111` ; `crates/whisper-worker-common/src/integrity.rs:1-22` ; `crates/whisper-worker-common/src/observation.rs:1-4`

Mesures : **631 production / 753 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-worker-common/Cargo.toml:13` |
| serde | externe | dependencies | {"workspace": true} | `crates/whisper-worker-common/Cargo.toml:14` |
| serde_json | externe | dependencies | {"workspace": true} | `crates/whisper-worker-common/Cargo.toml:15` |
| sha2 | externe | dependencies | {"workspace": true} | `crates/whisper-worker-common/Cargo.toml:16` |
| symphonia | externe | dependencies | {"workspace": true} | `crates/whisper-worker-common/Cargo.toml:17` |
| rusty_mp3 | externe | dependencies | "=0.8.0" | `crates/whisper-worker-common/Cargo.toml:18` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-worker-common/Cargo.toml:1-21`.

## Interfaces publiques et usages intercrates

| Consommateur | Contexte | Import observé | Source |
| --- | --- | --- | --- |
| whisper-worker-cpu | hors tests / cfg | `use whisper_worker_common::decoder::{MAX_WINDOW_SAMPLES, OUTPUT_RATE};` | `crates/whisper-worker-cpu/src/decoder.rs:4` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_worker_common::encoder::LiveMp3Encoder;` | `crates/whisper-worker-cpu/src/encoder.rs:1` |
| whisper-worker-cpu | hors tests / cfg | `use whisper_worker_common::integrity::verify_sha256;` | `crates/whisper-worker-cpu/src/integrity.rs:1` |
| whisper-worker-cpu | tests | `use whisper_worker_common::ipc::MAX_FRAME_BYTES;` | `crates/whisper-worker-cpu/src/ipc.rs:5` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_worker_common::decoder::{MAX_WINDOW_SAMPLES, OUTPUT_RATE};` | `crates/whisper-worker-gpu/src/decoder.rs:4` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_worker_common::encoder::LiveMp3Encoder;` | `crates/whisper-worker-gpu/src/encoder.rs:1` |
| whisper-worker-gpu | hors tests / cfg | `use whisper_worker_common::integrity::verify_sha256;` | `crates/whisper-worker-gpu/src/integrity.rs:1` |
| whisper-worker-gpu | tests | `use whisper_worker_common::ipc::MAX_FRAME_BYTES;` | `crates/whisper-worker-gpu/src/ipc.rs:5` |

Interfaces principales avec définitions et sites d’usage (pour desktop, entre cibles du même package) :

| Interface | Définition | Usage observé |
| --- | --- | --- |
| `decode_wav_windows` | `crates/whisper-worker-common/src/decoder.rs:64-143` | `crates/whisper-worker-cpu/src/decoder.rs:21-26` |
| `decode_requested_block` | `crates/whisper-worker-common/src/decoder.rs:16-62` | `crates/whisper-worker-gpu/src/decoder.rs:17-19` |
| `LiveMp3Encoder` | `crates/whisper-worker-common/src/encoder.rs:3-6` | `crates/whisper-worker-cpu/src/session/live.rs:4-254` |
| `read_frame` | `crates/whisper-worker-common/src/ipc.rs:6-57` | `crates/whisper-worker-gpu/src/ipc.rs:18-20` |
| `write_frame` | `crates/whisper-worker-common/src/ipc.rs:79-108` | `crates/whisper-worker-cpu/src/ipc.rs:22-24` |
| `verify_sha256` | `crates/whisper-worker-common/src/integrity.rs:4-19` | `crates/whisper-worker-cpu/src/session/import.rs:4-293` |
| `MemoryObserver` | `crates/whisper-worker-common/src/observation.rs:1` | `crates/whisper-worker-gpu/src/ipc.rs:7-16` |

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-worker-common/src/lib.rs:1-5`. Imports textuels, pas un graphe de symboles résolu.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | `crates/whisper-worker-common/src/decoder.rs:1` ; `crates/whisper-worker-common/src/integrity.rs:2` |
| Processus | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Threads/canaux | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Entrées/sorties standard | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| GPU/natif | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate::decoder::tests | Tests ou supports : définit `long_source_is_streamed_as_bounded_contiguous_windows` `crates/whisper-worker-common/src/decoder/tests.rs:18-68`, `lame_gapless_delay_and_padding_are_trimmed_once` `crates/whisper-worker-common/src/decoder/tests.rs:71-120`. | 0 | 307 |
| crate::decoder | Décode blocs/fenêtres WAV/MP3 avec Symphonia et un observateur injectable. `crates/whisper-worker-common/src/decoder.rs:1-438` | 436 | 2 |
| crate::encoder::tests | Tests ou supports : définit `packet_sequence_matches_original_encoder` `crates/whisper-worker-common/src/encoder/tests.rs:5-271`. | 0 | 271 |
| crate::encoder | Encode les blocs live en paquets MP3 avec rusty_mp3. `crates/whisper-worker-common/src/encoder.rs:1-61` | 59 | 2 |
| crate::integrity::tests | Tests ou supports : définit `accepts_expected_digest_and_rejects_mismatch_and_missing_file` `crates/whisper-worker-common/src/integrity/tests.rs:4-15`. | 0 | 15 |
| crate::integrity | Lit un fichier et compare son empreinte SHA-256 à la valeur attendue. `crates/whisper-worker-common/src/integrity.rs:1-22` | 20 | 2 |
| crate::ipc::tests | Tests ou supports : définit `frames_and_limits_are_preserved` `crates/whisper-worker-common/src/ipc/tests.rs:4-47`, `serialization_and_io_errors_are_propagated` `crates/whisper-worker-common/src/ipc/tests.rs:50-73`. | 0 | 73 |
| crate::ipc | Lit/écrit des trames JSON avec taille maximale et observation. `crates/whisper-worker-common/src/ipc.rs:1-111` | 109 | 2 |
| crate | Déclare les modules et interfaces de bibliothèque. `crates/whisper-worker-common/src/lib.rs:1-5` | 5 | 0 |
| crate::observation::tests | Tests ou supports : définit `observation_is_optional_and_feature_gated` `crates/whisper-worker-common/src/observation/tests.rs:9-27`, `decoder_observer_preserves_stages` `crates/whisper-worker-common/src/observation/tests.rs:30-70`. | 0 | 77 |
| crate::observation | Définit MemoryObserver, alias d’un pointeur de fonction de trace mémoire. `crates/whisper-worker-common/src/observation.rs:1-4` | 2 | 2 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| tests | externe | tests | crates/whisper-worker-common/src/decoder/tests.rs | `crates/whisper-worker-common/src/decoder.rs:437-438` |
| tests | externe | tests | crates/whisper-worker-common/src/encoder/tests.rs | `crates/whisper-worker-common/src/encoder.rs:60-61` |
| tests | externe | tests | crates/whisper-worker-common/src/integrity/tests.rs | `crates/whisper-worker-common/src/integrity.rs:21-22` |
| tests | externe | tests | crates/whisper-worker-common/src/ipc/tests.rs | `crates/whisper-worker-common/src/ipc.rs:110-111` |
| decoder | externe | hors tests / cfg | crates/whisper-worker-common/src/decoder.rs | `crates/whisper-worker-common/src/lib.rs:1` |
| encoder | externe | hors tests / cfg | crates/whisper-worker-common/src/encoder.rs | `crates/whisper-worker-common/src/lib.rs:2` |
| integrity | externe | hors tests / cfg | crates/whisper-worker-common/src/integrity.rs | `crates/whisper-worker-common/src/lib.rs:3` |
| ipc | externe | hors tests / cfg | crates/whisper-worker-common/src/ipc.rs | `crates/whisper-worker-common/src/lib.rs:4` |
| observation | externe | hors tests / cfg | crates/whisper-worker-common/src/observation.rs | `crates/whisper-worker-common/src/lib.rs:5` |
| tests | externe | tests | crates/whisper-worker-common/src/observation/tests.rs | `crates/whisper-worker-common/src/observation.rs:3-4` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate::decoder::tests — `crates/whisper-worker-common/src/decoder/tests.rs`

Tests ou supports : définit `long_source_is_streamed_as_bounded_contiguous_windows` `crates/whisper-worker-common/src/decoder/tests.rs:18-68`, `lame_gapless_delay_and_padding_are_trimmed_once` `crates/whisper-worker-common/src/decoder/tests.rs:71-120`.

Mesure : 0 production / 307 tests / 307 physiques. `crates/whisper-worker-common/src/decoder/tests.rs:1-307`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-common/src/decoder/tests.rs:1-307`.

Fonctions avec corps : `decode_wav_windows` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:2-7`; `decode_requested_block` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:8-10`; `long_source_is_streamed_as_bounded_contiguous_windows` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:18-68`; `lame_gapless_delay_and_padding_are_trimmed_once` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:71-120`; `write_silence_wav` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:122-145`; `capture_non_silent_pcm_baseline` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:148-210`; `invalid_range_missing_source_and_callback_failure_are_preserved` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:213-257`; `range_crossing_source_end_returns_available_pcm` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:260-285`; `existing_invalid_audio_source_reports_unsupported_format` [tests] `crates/whisper-worker-common/src/decoder/tests.rs:288-307`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-common/src/decoder/tests.rs:1-307`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/decoder/tests.rs:1-307`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::decoder — `crates/whisper-worker-common/src/decoder.rs`

Décode blocs/fenêtres WAV/MP3 avec Symphonia et un observateur injectable. `crates/whisper-worker-common/src/decoder.rs:1-438`

Mesure : 436 production / 2 tests / 438 physiques. `crates/whisper-worker-common/src/decoder.rs:1-438`

Types : `DecodeState` (struct, hors tests) `crates/whisper-worker-common/src/decoder.rs:145-163`.

Fonctions avec corps : `decode_requested_block` `crates/whisper-worker-common/src/decoder.rs:16-62`; `decode_wav_windows` `crates/whisper-worker-common/src/decoder.rs:64-143`; `new` `crates/whisper-worker-common/src/decoder.rs:166-195`; `process_packet` `crates/whisper-worker-common/src/decoder.rs:197-238`; `process_samples` `crates/whisper-worker-common/src/decoder.rs:240-283`; `emit_full_window` `crates/whisper-worker-common/src/decoder.rs:285-330`; `finish` `crates/whisper-worker-common/src/decoder.rs:332-394`; `decode_packets` `crates/whisper-worker-common/src/decoder.rs:397-430`; `_audio_buffer_ref_is_supported` `crates/whisper-worker-common/src/decoder.rs:433-435`.

Déclarations `pub` (visibilité sous conditions) : `pub const OUTPUT_RATE: u32 = 16_000;` `crates/whisper-worker-common/src/decoder.rs:13`; `pub const MAX_WINDOW_SAMPLES: usize = 80_000;` `crates/whisper-worker-common/src/decoder.rs:14`; `pub fn decode_requested_block(` `crates/whisper-worker-common/src/decoder.rs:16`; `pub fn decode_wav_windows(` `crates/whisper-worker-common/src/decoder.rs:64`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/decoder.rs:1-438`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::encoder::tests — `crates/whisper-worker-common/src/encoder/tests.rs`

Tests ou supports : définit `packet_sequence_matches_original_encoder` `crates/whisper-worker-common/src/encoder/tests.rs:5-271`.

Mesure : 0 production / 271 tests / 271 physiques. `crates/whisper-worker-common/src/encoder/tests.rs:1-271`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-common/src/encoder/tests.rs:1-271`.

Fonctions avec corps : `packet_sequence_matches_original_encoder` [tests] `crates/whisper-worker-common/src/encoder/tests.rs:5-271`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-common/src/encoder/tests.rs:1-271`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/encoder/tests.rs:1-271`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::encoder — `crates/whisper-worker-common/src/encoder.rs`

Encode les blocs live en paquets MP3 avec rusty_mp3. `crates/whisper-worker-common/src/encoder.rs:1-61`

Mesure : 59 production / 2 tests / 61 physiques. `crates/whisper-worker-common/src/encoder.rs:1-61`

Types : `LiveMp3Encoder` (struct, hors tests) `crates/whisper-worker-common/src/encoder.rs:3-6`.

Fonctions avec corps : `new` `crates/whisper-worker-common/src/encoder.rs:9-17`; `push_pcm` `crates/whisper-worker-common/src/encoder.rs:19-31`; `finish` `crates/whisper-worker-common/src/encoder.rs:33-39`; `drain_packets` `crates/whisper-worker-common/src/encoder.rs:41-51`; `default` `crates/whisper-worker-common/src/encoder.rs:55-57`.

Déclarations `pub` (visibilité sous conditions) : `pub struct LiveMp3Encoder {` `crates/whisper-worker-common/src/encoder.rs:3`; `pub fn new() -> Self {` `crates/whisper-worker-common/src/encoder.rs:9`; `pub fn push_pcm(&mut self, samples: &[i16]) -> Result<Vec<Vec<u8>>, String> {` `crates/whisper-worker-common/src/encoder.rs:19`; `pub fn finish(&mut self) -> Result<Vec<Vec<u8>>, String> {` `crates/whisper-worker-common/src/encoder.rs:33`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/encoder.rs:1-61`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::integrity::tests — `crates/whisper-worker-common/src/integrity/tests.rs`

Tests ou supports : définit `accepts_expected_digest_and_rejects_mismatch_and_missing_file` `crates/whisper-worker-common/src/integrity/tests.rs:4-15`.

Mesure : 0 production / 15 tests / 15 physiques. `crates/whisper-worker-common/src/integrity/tests.rs:1-15`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-common/src/integrity/tests.rs:1-15`.

Fonctions avec corps : `accepts_expected_digest_and_rejects_mismatch_and_missing_file` [tests] `crates/whisper-worker-common/src/integrity/tests.rs:4-15`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-common/src/integrity/tests.rs:1-15`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/integrity/tests.rs:1-15`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::integrity — `crates/whisper-worker-common/src/integrity.rs`

Lit un fichier et compare son empreinte SHA-256 à la valeur attendue. `crates/whisper-worker-common/src/integrity.rs:1-22`

Mesure : 20 production / 2 tests / 22 physiques. `crates/whisper-worker-common/src/integrity.rs:1-22`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-common/src/integrity.rs:1-22`.

Fonctions avec corps : `verify_sha256` `crates/whisper-worker-common/src/integrity.rs:4-19`.

Déclarations `pub` (visibilité sous conditions) : `pub fn verify_sha256(path: &Path, expected: &[u8; 32]) -> Result<(), String> {` `crates/whisper-worker-common/src/integrity.rs:4`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/integrity.rs:1-22`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc::tests — `crates/whisper-worker-common/src/ipc/tests.rs`

Tests ou supports : définit `frames_and_limits_are_preserved` `crates/whisper-worker-common/src/ipc/tests.rs:4-47`, `serialization_and_io_errors_are_propagated` `crates/whisper-worker-common/src/ipc/tests.rs:50-73`.

Mesure : 0 production / 73 tests / 73 physiques. `crates/whisper-worker-common/src/ipc/tests.rs:1-73`

Types : `Refused` (struct, tests) `crates/whisper-worker-common/src/ipc/tests.rs:51`; `Broken` (struct, tests) `crates/whisper-worker-common/src/ipc/tests.rs:57`.

Fonctions avec corps : `frames_and_limits_are_preserved` [tests] `crates/whisper-worker-common/src/ipc/tests.rs:4-47`; `serialization_and_io_errors_are_propagated` [tests] `crates/whisper-worker-common/src/ipc/tests.rs:50-73`; `serialize` [tests] `crates/whisper-worker-common/src/ipc/tests.rs:53-55`; `write` [tests] `crates/whisper-worker-common/src/ipc/tests.rs:59-61`; `flush` [tests] `crates/whisper-worker-common/src/ipc/tests.rs:62-64`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-common/src/ipc/tests.rs:1-73`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/ipc/tests.rs:1-73`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ipc — `crates/whisper-worker-common/src/ipc.rs`

Lit/écrit des trames JSON avec taille maximale et observation. `crates/whisper-worker-common/src/ipc.rs:1-111`

Mesure : 109 production / 2 tests / 111 physiques. `crates/whisper-worker-common/src/ipc.rs:1-111`

Types : `BoundedJsonFrame` (struct, hors tests) `crates/whisper-worker-common/src/ipc.rs:59`.

Fonctions avec corps : `read_frame` `crates/whisper-worker-common/src/ipc.rs:6-57`; `write` `crates/whisper-worker-common/src/ipc.rs:62-72`; `flush` `crates/whisper-worker-common/src/ipc.rs:74-76`; `write_frame` `crates/whisper-worker-common/src/ipc.rs:79-108`.

Déclarations `pub` (visibilité sous conditions) : `pub const MAX_FRAME_BYTES: usize = 1_048_576;` `crates/whisper-worker-common/src/ipc.rs:4`; `pub fn read_frame<T: DeserializeOwned>(` `crates/whisper-worker-common/src/ipc.rs:6`; `pub fn write_frame<T: Serialize>(` `crates/whisper-worker-common/src/ipc.rs:79`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/ipc.rs:1-111`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate — `crates/whisper-worker-common/src/lib.rs`

Déclare les modules et interfaces de bibliothèque. `crates/whisper-worker-common/src/lib.rs:1-5`

Mesure : 5 production / 0 tests / 5 physiques. `crates/whisper-worker-common/src/lib.rs:1-5`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-common/src/lib.rs:1-5`.

Fonctions avec corps : aucune relevée dans `crates/whisper-worker-common/src/lib.rs:1-5`.

Déclarations `pub` (visibilité sous conditions) : `pub mod decoder;` `crates/whisper-worker-common/src/lib.rs:1`; `pub mod encoder;` `crates/whisper-worker-common/src/lib.rs:2`; `pub mod integrity;` `crates/whisper-worker-common/src/lib.rs:3`; `pub mod ipc;` `crates/whisper-worker-common/src/lib.rs:4`; `pub mod observation;` `crates/whisper-worker-common/src/lib.rs:5`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/lib.rs:1-5`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::observation::tests — `crates/whisper-worker-common/src/observation/tests.rs`

Tests ou supports : définit `observation_is_optional_and_feature_gated` `crates/whisper-worker-common/src/observation/tests.rs:9-27`, `decoder_observer_preserves_stages` `crates/whisper-worker-common/src/observation/tests.rs:30-70`.

Mesure : 0 production / 77 tests / 77 physiques. `crates/whisper-worker-common/src/observation/tests.rs:1-77`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-worker-common/src/observation/tests.rs:1-77`.

Fonctions avec corps : `observe` [tests] `crates/whisper-worker-common/src/observation/tests.rs:4-6`; `observation_is_optional_and_feature_gated` [tests] `crates/whisper-worker-common/src/observation/tests.rs:9-27`; `decoder_observer_preserves_stages` [tests] `crates/whisper-worker-common/src/observation/tests.rs:30-70`; `observe_decoder` [tests] `crates/whisper-worker-common/src/observation/tests.rs:72-77`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-worker-common/src/observation/tests.rs:1-77`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/observation/tests.rs:1-77`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::observation — `crates/whisper-worker-common/src/observation.rs`

Définit MemoryObserver, alias d’un pointeur de fonction de trace mémoire. `crates/whisper-worker-common/src/observation.rs:1-4`

Mesure : 2 production / 2 tests / 4 physiques. `crates/whisper-worker-common/src/observation.rs:1-4`

Types : `MemoryObserver` (type, hors tests) `crates/whisper-worker-common/src/observation.rs:1`.

Fonctions avec corps : aucune relevée dans `crates/whisper-worker-common/src/observation.rs:1-4`.

Déclarations `pub` (visibilité sous conditions) : `pub type MemoryObserver = fn(&str, serde_json::Value);` `crates/whisper-worker-common/src/observation.rs:1`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-worker-common/src/observation.rs:1-4`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 0 | 0 |
| cfg_feature | 28 | 4 |
| cfg_test | 0 | 5 |
| command_new | 0 | 0 |
| expect | 0 | 1 |
| thread_spawn | 0 | 0 |
| unsafe | 0 | 0 |
| unwrap | 0 | 80 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 1 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 0 sur plages hors tests et 1 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::too_many_lines | 0 | 1 |
