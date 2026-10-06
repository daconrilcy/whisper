# Autorisation utilisateur — périmètre L-WHISPER-01 / 21 chemins

Source directe : demande utilisateur dans cette conversation, 2026-10-06 : « 5. Enregistrer l’autorisation explicite des 21 chemins, puis actualiser l’inventaire et les hashes. 6. Refaire le préflight et check-state --lot L-WHISPER-01 sur ce périmètre révisé. »

Cette instruction autorise l’enregistrement comme périmètre exact d’implémentation de L-WHISPER-01 des 21 chemins suivants, repris sans glob de `candidates/P-WHISPER-06/01_LOTS.md` et de la source acceptée ARCH-L01-INTEGRATION v2 (`transports/T-WHISPER-ARCH-L01-INTEGRATION-02/CHANGE-L01-INTEGRATION-01-raw.md`, SHA256 `1c8675f6d1e0f18e05556e8e3118d824e1c95591aca37e8d4e6823733787ea27`). Cette portée autorise le périmètre documentaire et l’enregistrement d’autorisation; elle ne déclare aucun contenu futur présent ni hash fictif.

```text
Cargo.lock
crates/whisper-adapters/Cargo.toml
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/decoder.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/import_cpu.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/ipc.rs
crates/whisper-core/src/lib.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/import_contract.rs
crates/whisper-desktop/Cargo.toml
crates/whisper-desktop/src/main.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-worker-cpu/Cargo.toml
crates/whisper-worker-cpu/src/decoder.rs
crates/whisper-worker-cpu/src/ipc.rs
crates/whisper-worker-cpu/src/main.rs
crates/whisper-worker-cpu/src/native_engine.rs
```

Lot : `L-WHISPER-01`. Candidat PLANS de référence : P-WHISPER-06, digest `5759cb207f39fe62fee921321dd90275a67a8d687133cf6fd23afd6ac09e3a93`, revu CLEAN par un reviewer indépendant.
