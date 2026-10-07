# Baseline de source L04 — D-WHISPER-20

Révision observée : `ffd93125604be5dc6439587b7edad4f420f5cccc` (branche `main`), le 2026-10-07.

Cette baseline est le D0 propre au lot L04 après les lots L00–L03. Elle complète la baseline de conception D19 sans la remplacer et lie au manifeste DESIGN les dix fichiers déjà présents. Les cinq créations futures sont intentionnellement absentes et ne reçoivent aucun hash. Ce document ne constitue pas le préflight ni une qualification produit.

Le snapshot P24 décrivait le même périmètre au `c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b`. `git diff --name-only c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b ffd93125604be5dc6439587b7edad4f420f5cccc -- crates Cargo.lock Cargo.toml rust-toolchain.toml` est vide : entre ces révisions, seuls des documents hors du périmètre code ont changé. Les hashes observés ci-dessous égalent les snapshots recopiés sans conversion depuis P24.

## Allowlist L04 (15 chemins)

- `crates/whisper-adapters/src/supervisor.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-adapters/tests/worker_control.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-core/tests/compute_policy.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-worker-cpu/src/native_engine.rs` — PRÉSENT; SHA-256 `9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-worker-cpu/src/native_engine.rs.txt`
- `crates/whisper-worker-gpu/src/native_engine.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-adapters/src/lib.rs` — PRÉSENT; SHA-256 `9b71f5efe1d6669ca6e38e865915b6e4bfb6504558e076f520eaf86f20d09174`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-adapters/src/lib.rs.txt`
- `crates/whisper-adapters/src/worker_ipc.rs` — PRÉSENT; SHA-256 `4f1bfa7a9bdf0811046204831224a348901dfc36b64cec225ebab53ed284e2c2`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-adapters/src/worker_ipc.rs.txt`
- `crates/whisper-core/src/application.rs` — PRÉSENT; SHA-256 `47c32fcc0f9d2709670723ffea636a0324bdef51a62f9d0c9b35dbc4f2294c69`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-core/src/application.rs.txt`
- `crates/whisper-core/src/compute_policy.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-core/src/lib.rs` — PRÉSENT; SHA-256 `5b047a792ee30b73fed1e3cab2c8bc1a47b8f41f781ceae750905cbd27607d3a`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-core/src/lib.rs.txt`
- `crates/whisper-desktop/src/root.rs` — PRÉSENT; SHA-256 `9a3713112e3dfc3872e15e3cb581c7c6a53222e5f62864144ba4d111caa98599`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-desktop/src/root.rs.txt`
- `crates/whisper-desktop/src/ui.rs` — PRÉSENT; SHA-256 `148900588d475028b2c0c27c65ed05116dd93aecf62568c78cef6a554b913648`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-desktop/src/ui.rs.txt`
- `crates/whisper-worker-gpu/src/main.rs` — PRÉSENT; SHA-256 `047430149342e5fec4e77d8b9009ee377af8c0b3c50a9c96b3fb52536d96f953`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-worker-gpu/src/main.rs.txt`
- `crates/whisper-worker-gpu/Cargo.toml` — PRÉSENT; SHA-256 `5ef21c0437965d8ea28c01bdd33f5c5d3ec867e2498ab4329b3a24ad54553b89`; snapshot DESIGN `sources/L04-config-baseline/crates__whisper-worker-gpu__Cargo.toml.txt`
- `Cargo.lock` — PRÉSENT; SHA-256 `9d714d4315a5dd63865faa6bc8eb9bcb84eb9abdfdd2182a1fafbb4c2b60b67f`; snapshot DESIGN `sources/L04-config-baseline/Cargo.lock.txt`

## Inventaire et reprise

`code_state.baseline` et `code_state.current` au préflight L04 doivent contenir exactement les dix fichiers présents ci-dessus, avec leurs snapshots D20. `ledger` démarre vide pour ce D0 L04. Les cinq fichiers absents restent hors baseline et current jusqu’à leur création autorisée.

L’identité d’observation de P24 (`c75ae…`) est conservée comme historique. L’autre inventory archivé (`1310bbcd…`) demeure inchangé. Cette observation D20, datée au HEAD courant, est la référence de ce lot. La comparaison ne prétend pas que le worktree global est propre.

L’autorisation de préparation de cette proposition de succession DESIGN est capturée dans `USER_AUTHORIZATION_D20_PROPOSAL.md`; l’autorisation distincte d’implémentation et de préflight est capturée dans `USER_AUTHORIZATION_L04.md`.
