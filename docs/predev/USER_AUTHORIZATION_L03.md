# Autorisation utilisateur — implémentation L-WHISPER-03

Source directe : instruction utilisateur reçue dans la conversation Codex active le 2026-10-06.

> « Tu peux lancer L03 »

Cette instruction autorise l’implémentation du lot L-WHISPER-03 après réussite du préflight ciblé. L’autorisation est bornée à l’allowlist P-WHISPER-10 ci-dessous. Elle n’autorise aucun autre lot, aucune campagne de qualification non prévue, ni aucune modification des artefacts de conception/plans prêts.

## Chemins autorisés (28)

- `Cargo.lock`
- `crates/whisper-adapters/Cargo.toml`
- `crates/whisper-adapters/src/archive.rs`
- `crates/whisper-adapters/src/capture.rs`
- `crates/whisper-adapters/src/journal.rs`
- `crates/whisper-adapters/src/lib.rs`
- `crates/whisper-adapters/src/recovery.rs`
- `crates/whisper-adapters/src/staging.rs`
- `crates/whisper-adapters/src/vad.rs`
- `crates/whisper-adapters/src/worker_ipc.rs`
- `crates/whisper-adapters/tests/durability.rs`
- `crates/whisper-adapters/tests/live_archive.rs`
- `crates/whisper-adapters/tests/live_capture.rs`
- `crates/whisper-adapters/tests/vad_contract.rs`
- `crates/whisper-core/src/application.rs`
- `crates/whisper-core/src/domain.rs`
- `crates/whisper-core/src/ipc.rs`
- `crates/whisper-core/src/lib.rs`
- `crates/whisper-core/src/ports.rs`
- `crates/whisper-core/tests/live_contract.rs`
- `crates/whisper-core/tests/scheduler_contract.rs`
- `crates/whisper-desktop/src/root.rs`
- `crates/whisper-desktop/src/ui.rs`
- `crates/whisper-worker-cpu/Cargo.toml`
- `crates/whisper-worker-cpu/src/encoder.rs`
- `crates/whisper-worker-cpu/src/ipc.rs`
- `crates/whisper-worker-cpu/src/main.rs`
- `crates/whisper-worker-cpu/tests/live_worker.rs`
