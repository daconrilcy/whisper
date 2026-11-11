Avancée : les trois changements de PETITS-LOTS-2 sont réalisés sur `petits-lots-2`, relus et validés.
Blocage : aucun ; le premier N1 sans chemin de modèle a échoué au préflight de huit tests, puis le N1 correctement paramétré a passé.
Décision : conserver le worktree et les preuves jusqu’à fusion utilisateur ; aucun merge ni push effectué.

# PETITS-LOTS-2 — bilan figé

| Point | Changement et preuve |
|---|---|
| 1 — harness CPU | Dans `crates/whisper-worker-cpu/tests/support/autonomous_harness.rs`, retrait de l’inclusion de `engine_cache.rs` et du stub `CpuEngine::load`. Type factice conservé. Aucun `src/`, `#[allow]` ou autre entrée `dead_code` modifié. Inventaire gardé avant/après : 21 identités, 4 résumés de cibles, identiques. Revue `rust_review_coordinator` : **CLEAN**. |
| 2 — `.gitignore` | Ajout de `/evidence/` à la racine. `git ls-files evidence/` vide. Relecture générique : **CLEAN**. |
| 3 — `AGENTS.md` | Ajout de la règle demandée sous « Préparation et architecture ». Relecture générique : **CLEAN**. |

**Clippy CPU gardé**, avant et après : `cargo clippy --offline --locked -p whisper-worker-cpu --all-targets --message-format=json` avec les quatre `-W` structurels prescrits. Code 0 aux deux runs ; `cached_cpu_engine` et le stub du harness disparaissent, diagnostics `dead_code` attendus retirés, émissions 19→17 et clés d’avertissement normalisées 14→12, aucun nouveau site. Cliquet structurel : **PASS**, 10 sites.

**N1 final gardé**, avec `-WithoutCargoTargetDir` et le modèle personnel validé par SHA-256 : **944 réussis / 0 échec / 129 ignorés** ; `CARGO_TARGET_DIR` absent du processus Cargo. La première invocation, sans `-PersonalParakeetModelPath`, a échoué sur le préflight des tests qui requièrent ce modèle ; la reprise autorisée avec son chemin vérifié a passé. SelfTest de la garde : **PASS**, 72 positifs / 106 négatifs.

Preuves brutes (12 journaux, arborescence et SHA-256 vérifiés) : `evidence/PETITS-LOTS-2-validation.zip`, 86 700 octets, SHA-256 `ee9a4ba6803bc1971d1c881e7a6c362bd76d3aba1c452c52f64109aeb561b7b1`. `git diff --check` : PASS. Pas de Clippy workspace, conformément au périmètre CUDA.

Commits du lot : `57272433` (point 2), `c5e6077f` (point 3), puis le commit point 1 et ce bilan. Nettoyage proposé après fusion utilisateur : supprimer seulement `C:\wt\petits-lots-2\target\parakeet-tests\garde-n1-categories`; conserver la cible Cargo partagée, le cache Cargo persistant, l’archive de preuves et le worktree jusqu’à la décision de fusion.

Suivi du nettoyage : [PETITS-LOTS-2-SUIVI-NETTOYAGE.md](PETITS-LOTS-2-SUIVI-NETTOYAGE.md).
