# Autorisation utilisateur — L-WHISPER-01

Source : instruction directe de HUMAN-USER reçue dans la conversation Codex active le 2026-10-05 : « persister une autorisation explicite de L01 couvrant ses chemins ». Cette instruction autorise explicitement l’implémentation du lot L-WHISPER-01, après passage positif du préflight et de `check-state --lot L-WHISPER-01`.

## Périmètre autorisé

Le scope reprend les chemins L01 enregistrés et les surfaces d’ownership du plan P-WHISPER-02. L’implémentation est limitée à :

- `crates/whisper-adapters/src/archive.rs`
- `crates/whisper-adapters/src/decoder.rs`
- `crates/whisper-adapters/src/worker_ipc.rs`
- `crates/whisper-adapters/tests/import_cpu.rs`
- `crates/whisper-core/tests/import_contract.rs`
- `crates/whisper-desktop/src/ui.rs`
- `crates/whisper-worker-cpu/**`

Aucun autre chemin n’est autorisé. Le ledger L01, V-IMPORT et V-UI seront produits après ce gate.
