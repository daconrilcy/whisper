# Préflight initial L-WHISPER-01 — périmètre révisé

Date : 2026-10-06. Candidat PLANS : P-WHISPER-06, digest `5759cb207f39fe62fee921321dd90275a67a8d687133cf6fd23afd6ac09e3a93` (CLEAN documentaire ; état READY à promouvoir contrôlé séparément). Autorisation : `transports/T-WHISPER-P06-REVIEW-AUTH-01/user-authorization-21-paths.md`. Lot : L-WHISPER-01, liste exacte sans glob conforme à INTEGRATION v2.

## Dépôt

- Racine : `C:\dev\whisper`
- HEAD : `5a5befa4e506d3da6a042d51f3a2bd0b49794adc`
- `git status --short` :

```text
?? docs/predev/candidates/P-WHISPER-03/
?? docs/predev/candidates/P-WHISPER-04/
?? docs/predev/candidates/P-WHISPER-05/
?? docs/predev/candidates/P-WHISPER-06/
?? docs/predev/transports/T-WHISPER-P06-REVIEW-AUTH-01/
?? docs/predev/transports/T-WHISPER-PW-P04-CORRECTIONS-01/
?? target/
```

- `git diff --stat` : vide au moment de la capture. Les candidats P03–P06, transports et `target/` sont untracked et ont été préservés.

## Inventaire L01 — 21 chemins exacts

| Chemin | État et hash courant |
|---|---|
| `Cargo.lock` | Présent, SHA256 `447abf23220b0a3cb5c58ea2222f0e04355acb5f2d179c33aacc771c8ba616b8` |
| `crates/whisper-adapters/Cargo.toml` | Présent, SHA256 `812d0dbc03e1359dc1bd19f5576d03a91dcaa0878022380e2f7e61901c75f344` |
| `crates/whisper-adapters/src/archive.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-adapters/src/decoder.rs` | Présent, SHA256 `ea7564ca660951d1b763633dab92875340e0897b0c5e505fdf7f6cec31be3506` |
| `crates/whisper-adapters/src/lib.rs` | Présent, SHA256 `6329856cb79a1ca8712dc6f549ce7d003abbc1ed7452a89be496b01d0750fbff` |
| `crates/whisper-adapters/src/worker_ipc.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-adapters/tests/import_cpu.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-core/src/application.rs` | Présent, SHA256 `f9aa136fccebe741f4ff8e3bb1c0f2a04c0562d1e9d8fedd09115c79f6e88cb0` |
| `crates/whisper-core/src/ipc.rs` | Présent, SHA256 `9c64a6af41a1fb5fe22ffb8624441fb9e530e5c11b06328ffb2b8f6269d22e10` |
| `crates/whisper-core/src/lib.rs` | Présent, SHA256 `64c5ba268ba38316e2baca0a943cc064da3a6b04b5570e129964a706225aa0f5` |
| `crates/whisper-core/src/ports.rs` | Présent, SHA256 `03fdef88124b4cc7c4cb7a65d76c3cc92cf067be774026b88b162469401d9b03` |
| `crates/whisper-core/tests/import_contract.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-desktop/Cargo.toml` | Présent, SHA256 `0528886e5b8575b30ada28910a50dbfe9519a7c364d33043161e046de9121d15` |
| `crates/whisper-desktop/src/main.rs` | Présent, SHA256 `04818ef9fdc8d37d0166b71b989da8d63b69a8b372af480c425e24f2aff7e38b` |
| `crates/whisper-desktop/src/root.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-desktop/src/ui.rs` | Présent, SHA256 `9317b88831780f53cc69aefdc8fb0b3c5d196fa7c9729233a8840c5b5064e4c8` |
| `crates/whisper-worker-cpu/Cargo.toml` | Présent, SHA256 `3fc393931d7d16f705b7d37fee9b0821b21f48cd3b696af2394960c3aa159bb9` |
| `crates/whisper-worker-cpu/src/decoder.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-worker-cpu/src/ipc.rs` | Absent, futur ; aucun hash attribué |
| `crates/whisper-worker-cpu/src/main.rs` | Présent, SHA256 `047430149342e5fec4e77d8b9009ee377af8c0b3c50a9c96b3fb52536d96f953` |
| `crates/whisper-worker-cpu/src/native_engine.rs` | Absent, futur ; aucun hash attribué |

Comptage : 13 présents et hashés, 8 absents/futurs, 21 au total. Les futurs n’ont aucun hash prérempli. `code_state.current` reste limité aux sorties L00 du ledger, rehashées ci-dessous ; aucun nouveau code n’est déclaré produit.

## Ledger L00 et état courant suivi

| Chemin | SHA256 recalculé | Correspond au ledger L00 |
|---|---|---|
| `crates/whisper-adapters/src/decoder.rs` | `ea7564ca660951d1b763633dab92875340e0897b0c5e505fdf7f6cec31be3506` | oui |
| `crates/whisper-desktop/src/ui.rs` | `9317b88831780f53cc69aefdc8fb0b3c5d196fa7c9729233a8840c5b5064e4c8` | oui |
| `crates/whisper-worker-cpu/src/main.rs` | `047430149342e5fec4e77d8b9009ee377af8c0b3c50a9c96b3fb52536d96f953` | oui |

## Contrôles non exécutés / étapes

- `check-build-environment` avec `RUSTUP_TOOLCHAIN=1.98.1` et `RUSTUP_AUTO_INSTALL=0` : NOT RUN ; les résultats restent absents.
- `rustup target list --installed --toolchain 1.98.1` : NOT RUN.
- Contrôle des DLL/SDK/linkage : NOT RUN.
- Préflight d’exécution/fixtures/modèle/import réel : NOT RUN.
- `check-state --lot L-WHISPER-01` sur le checkpoint révisé : À EXÉCUTER ; aucun PASS déclaré. Le défaut de racines séparées du contrôleur peut imposer la vue unifiée hashée décrite dans P06.
- Builds, tests produit, qualification native : NOT RUN.

## Résultats des contrôles du 2026-10-06

Matrice P06 `build-environment-L01.json`, SHA256 `baf6c8a0afbcde35aaf9fae0266576b40ac3ab8941186e931ddc2bead1ea267d`, exécutée avec `RUSTUP_TOOLCHAIN=1.98.1`, `RUSTUP_AUTO_INSTALL=0` : `ok=false`, `status=BLOCKED`. Rustc 1.98.1 READY ; Cargo 1.98.1 READY ; `RUSTUP_TOOLCHAIN` configurée ; `rust-toolchain.toml` READY ; `Cargo.lock` READY. MSVC `cl` MISSING ; CMake MISSING ; `LIBCLANG_PATH` MISSING. Aucune installation ou correction automatique. Précondition environnement bloquante avant toute modification produit. `rustup target list --installed --toolchain 1.98.1` non exécutée. SDK/linkage/libclang chargeable non vérifiés.

Essai contrôleur sur racines originales :

```text
python -B ... predev_control.py check-state --root docs/predev --code-root . --state docs/predev/state.pending.json --lot L-WHISPER-01
predev-control: file missing: Cargo.lock
```

Contrôle réussi sur la vue temporaire hashée avant promotion, même candidat/lots/checkpoint pending : `ok=true`, `phase=IMPLEMENTATION`, lot `L-WHISPER-01`, D19 digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`, PLANS P06 digest `5759cb207f39fe62fee921321dd90275a67a8d687133cf6fd23afd6ac09e3a93`, `observed_drift={}`. La vue avait 2 480 documents copiés à hash identique et 23 sorties CODE L00 copiées à hash identique ; aucune collision de chemins. Le résultat est uniquement le contrôle documentaire/state et n’efface pas le blocage environnemental. Répertoire et inventaire de vue : `C:/Users/cyril/.codex/tmp/whisper-l01-gate-20261006/view-manifest.json` (refresh après mise à jour de ce rapport).
