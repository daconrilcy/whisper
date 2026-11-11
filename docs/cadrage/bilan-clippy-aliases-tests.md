Avancée : les sept diagnostics `clippy::type_complexity` ciblés ont disparu après introduction de quatre alias et réutilisation de `PublicationGateRegistry`.
Blocage : la Clippy workspace de resserrement n’a pas été exécutée, car elle sélectionne le worker GPU/CUDA suspendu.
Décision : référence resserrée de sept occurrences selon D-04 ; bilan et preuves complétés, sans merge ni push.

# Bilan — Alias de types des supports de test IPC

**Référence :** [plan-clippy-aliases-tests.md](plan-clippy-aliases-tests.md) et [evo-clippy-aliases-tests.md](evo-clippy-aliases-tests.md).

## Résultat

Les types imbriqués des supports de test IPC ont été nommés dans les trois fichiers prévus. Quatre alias locaux ont été ajoutés ; l’alias existant `PublicationGateRegistry` a été réutilisé. Les types sous-jacents, l’ordre des tuples, les scénarios et les statuts des tests sont préservés. La comparaison Clippy JSON par site donne sept diagnostics ciblés avant et zéro après ; le cliquet passe avec 194 sites. La revue indépendante a conclu **CLEAN**, sans constat ni vérification supplémentaire.

## Vérifications

- SelfTest de `ci/guard-level1.ps1` : PASS (72 cas positifs, 106 négatifs).
- SelfTest PE de `ci/check-worker-imports.ps1` : PASS (4 positifs, 15 négatifs).
- Clippy gardée `whisper-adapters --all-targets` avec les quatre lints structurels : PASS avant et après ; 7 sites ciblés avant, 0 après ; baseline inchangée.
- `cargo fmt --check` gardé : PASS.
- `cargo test -p whisper-adapters` gardé avec le modèle Parakeet validé : PASS.
- Tests des membres par défaut : PASS.
- `whisper-worker-cpu`, `whisper-worker-parakeet-cpu` et `whisper-worker-common` : harnais vérifiés, inventaires et tests gardés PASS.
- `cargo test --no-fail-fast` gardé sans variable `CARGO_TARGET_DIR` : PASS ; la garde a contrôlé `target_directory` et `CARGO_TARGET_DIR_child=False`.
- `git diff --check` : PASS.

Modèle utilisé pour les scénarios Parakeet : `C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf`, SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`.

## Livraison et limites

Les preuves de campagne (81 fichiers, 3 164 897 octets) sont archivées sous `C:/wt/clippy-aliases-tests/evidence/clippy-aliases-tests/scratch/`; chemins relatifs et SHA-256 vérifiés. La cible neuve propre au lot a été supprimée après vérification de son chemin et de l’absence de processus Cargo/Rust l’utilisant ; le LotRoot, les preuves et les caches persistants/shared ont été conservés.

Le candidat a été produit dans `C:/wt/clippy-aliases-tests`, branche `codex/clippy-aliases-tests`, depuis la base `3589a8d6685f88f6ae851ef378db720fccc374a2`. Aucun test GPU, build CUDA, build de DLL native ou essai dans la fenêtre Windows n’a été exécuté. Ces validations restent hors du plan.

## Complément de clôture — resserrement manuel de la référence

**Décision :** D-04, demande de resserrement manuel sur les sept occurrences retirées. L’en-tête de `ci/clippy-structure-baseline.tsv` porte `tightened=b445ac36 clippy-aliases-tests -7 type_complexity`. Le total passe de 248 à **241 occurrences** ; la référence contient 241 clés et 241 occurrences après suppression des entrées ciblées.

Identifiants retirés (six clés, sept occurrences) :

- `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/service/mod.rs|) -> (` (1)
- `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs|gates: Arc<` (1)
- `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs|samples_rx: Receiver<` (1)
- `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs|) -> (` (2)
- `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs|controlled_capture_blocks: Option<` (1)
- `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs|fn held_final_a_worker` (1)

Il reste **1 identifiant de type 3** (sans nom de fonction) : `clippy::type_complexity|whisper-worker-parakeet-cpu|crates/whisper-worker-parakeet-cpu/src/transcription.rs|) -> Result<` (1 occurrence).

La N1 gardée a réussi avec **944 / 0 / 129** (réussis / échecs / ignorés). L’archive [CLIPPY-ALIASES-TESTS-20261111.zip](../architecture/raw/CLIPPY-ALIASES-TESTS-20261111.zip) contient 81 membres, 3 164 897 octets source ; les chemins relatifs et SHA-256 de chaque membre ont été vérifiés.

La comparaison workspace demandée n’a pas été exécutée. `whisper-worker-gpu` active `whisper-rs` avec la feature `cuda`; l’exécution `clippy --workspace --all-targets` sélectionnerait cette configuration, hors de la validation CPU gardée et sous suspension CUDA/native. Le total **241** est le compte de la référence après le retrait manuel ; le critère workspace « code 0, zéro nouveau site » reste à vérifier après arbitrage de cette limite.
