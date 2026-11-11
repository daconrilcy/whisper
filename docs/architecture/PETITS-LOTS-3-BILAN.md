Avancée : un site de test structurel supprimé et la règle « pas de push agent » ajoutée.
Blocage : aucun.
Décision : fusion de la branche `petits-lots-3` par l'utilisateur.

# PETITS-LOTS-3 — bilan

Phase A : baseline initiale de 241 clés. Par lint/package : cognitive — adapters 35, core 2, desktop 1, parakeet-cpu 2 ; arguments — adapters 5, worker-cpu 1, worker-gpu 1, parakeet-cpu 1 ; lignes — adapters 150, core 14, desktop 11, worker-common 1, worker-cpu 5, worker-gpu 4, parakeet-cpu 7 ; type — parakeet-cpu 1. Par emplacement : `src/` 82, `tests/` 157, `tests/support/` 2.

Site traité : `clippy::too_many_arguments|whisper-worker-parakeet-cpu|crates/whisper-worker-parakeet-cpu/tests/native_import.rs|fn request_with_hash` ; regroupement de ses paramètres de test dans `RequestDetails<'a>`. La clé type 3 restante, dans `crates/whisper-worker-parakeet-cpu/src/transcription.rs`, est en production et a été écartée. Baseline : 241 → 240, retrait exact d'une clé.

Clippy gardé, package `whisper-worker-parakeet-cpu`, `--all-targets` et quatre lints structurels : avant PASS, 15 clés/sites contre 241 ; après PASS, 14 contre 240. Aucun avertissement non structurel avant ou après. Inventaire `cargo test -- --list` : 37 identités avant et après, ensembles identiques.

Validation : N1 gardé `cargo test --no-fail-fast` avec `-WithoutCargoTargetDir` et modèle personnel vérifié par SHA-256 : 944 réussis / 0 échec / 129 ignorés ; `CARGO_TARGET_DIR_child=False`. SelfTest garde PASS, 72 positifs / 106 négatifs. `cargo fmt --check` et `git diff --check` PASS. Revue de la règle : oui, claire et cohérente ; revue Rust indépendante : CLEAN.

Preuves brutes : `evidence/PETITS-LOTS-3-validation.zip`, 53 123 octets, SHA-256 `D046E92E525B4D6813769DBED394AFA10943D0F8506816B8FA8742A929465935`.

Commits : point 1 `28d11f3c0c55b459a8d9a2a14456478d08a499c9` (`docs: prohibit agent pushes`) ; point 2 `578d985a3ad60e07c5fd842c19f26b08ed6950fb` (`test: reduce structural clippy baseline`). Aucun merge ni push.

Suivi : aucun
