Avancée : correctif validé sur d710d1e8 ; tests workspace réussis : 686 / 0 / 148 ; preuve 4 acquise.
Blocage : aucun pour la vérification demandée.
Décision : bilan et preuves actualisés après une seule exécution autorisée ; aucune correction ni push.

# GARDE-N1 — catégories de commandes Cargo

Branche `garde/n1-categories`, worktree `C:/dev/whisper/target/agent-worktrees/garde-n1-categories`, base `refacto/tests-autonomes` **939fbbbe**. Plan initial : **09520288** ; garde livrée : **a06ad99c**. Le correctif de test distinct a été planifié dans **92fa308a**, puis livré dans **d710d1e8**. Le candidat était propre avant la présente validation du 1er novembre 2026.

| Catégorie | Commandes | Conditions |
| --- | --- | --- |
| Lecture | `tree`, `metadata`, `fmt --check` | arguments explicitement autorisés ; environnement isolé |
| Compilation | `check`, `build`, `test --no-run`, `clippy` | workspace ou un/plusieurs membres `-p`, release permis |
| Tests | `test` | ni ignored/include-ignored, ni feature native-tests |

Les membres `-p` proviennent de metadata protégé. Cible par défaut : `C:/dev/whisper/target/parakeet-tests/cargo-shared/`. `-LotRoot` désigne une racine contrôlée dans le worktree ; `-FreshTarget <nom>` impose une cible neuve sous cette racine. `-UnsetEnv` accepte seulement CUDA_PATH, CUDA_PATH_V*, GGML_*, WHISPER_* et PARAKEET_*. Les contrôles de chemins/liens, espace libre, CARGO_HOME/TEMP/TMP/LOCALAPPDATA isolés, stdout/stderr, codes de sortie et offline/locked sont conservés ; aucune surveillance de fichiers. `-DryRun` peut appeler metadata protégé, mais n’exécute pas la commande demandée. Structure est conservé, route vérifiée en DryRun.

Refus par défaut de tout argument inconnu ; refus explicites : run/install/uninstall/update/publish/clean/add/remove/fix/generate-lockfile/vendor et commandes externes non listées ; --fix, --allow-dirty, --all-features, native-tests (variantes qualifiées comprises), recette-hooks en release, --ignored, --include-ignored, --config, -Z, --target-dir et manifest-path hors worktree.

## Preuves 1 à 4

| Preuve | Résultat et référence |
| --- | --- |
| 1 — auto-tests | **PASS**, 20 positifs / 49 négatifs ; `-Mode SelfTest` relancé avant les tests, code 0. Aucun Cargo dans ces auto-tests. [Résultat](raw/garde-n1/selftest.txt), [corrections](raw/garde-n1/correction-cycle1.txt). |
| 2 — R8 | **PASS**, six commandes du diagnostic plus D/UnsetEnv, cibles neuves A2/B2/D, commandes demandées non exécutées. [Sorties](raw/garde-n1/r8-dry-runs.txt), invocations dans le journal des corrections. Preuve antérieure conservée ; le correctif Rust ne modifie pas la garde. |
| 3 — tree réel | **PASS**, `tree --offline --locked -p whisper-worker-cpu -e features -i whisper-rs-sys`, code 0 sur cible partagée. [stdout](raw/garde-n1/tree.stdout.log), [stderr](raw/garde-n1/tree.stderr.log). Preuve antérieure conservée. |
| 4 — tests workspace | **PASS sur d710d1e8**, `test --offline --locked --workspace`, code **0** : **686 réussis / 0 échec / 148 ignorés**, 73 groupes, total 834. [stdout](raw/garde-n1/workspace-test.stdout.log), [stderr](raw/garde-n1/workspace-test.stderr.log). |

Invocation exacte depuis le worktree : `& ./ci/guard-level1.ps1 -CargoArgs @('test','--workspace')`. La garde applique offline/locked, le préflight et l’environnement isolé. Le journal stderr confirme les exécutables sous la cible partagée canonique. Une seule exécution a été lancée après l’autorisation utilisateur de validation du correctif ; aucune nouvelle correction. Le test `native_executor_backend_changes_only_after_the_prior_instance_is_reaped` est explicitement **ok** (stdout, ligne 252). La compilation conserve des avertissements ; ce passage n’est pas un contrôle Clippy.

La référence initiale 687/0/147 avait été remplacée par **686/0/148** avec accord utilisateur après confirmation de l’antériorité de l’ignore manuel `capture_encoder_baseline` : [bilan de la base](TESTS-AUTONOMIE-INVENTAIRE.md), lignes 1 et 21.

## Correctif, revue et historique

Le test exigeait un événement au premier polling malgré le démarrage asynchrone du remplaçant. Le correctif **d710d1e8** remplace cet appel par une attente de cinq secondes maximum, avec pause de deux millisecondes sur `None`. Le premier événement, y compris une erreur, reste soumis aux assertions existantes Ready/JobId/backend CPU. La vérification de l’exécuteur et le scénario NativeRouteBusy sont conservés. Seul `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs` a été modifié côté Rust ; aucun code de production, manifeste ou lockfile modifié.

Implémentation déléguée et revue indépendante : R1–R7 de la garde fermés après deux cycles ; documentation relue. Le correctif distinct du test a reçu une revue statique **CLEAN**, sans correction supplémentaire. Son exécution était alors interdite ; la présente autorisation apporte maintenant la preuve runtime manquante, sans transformer la validation en nouveau verdict de revue.

Avant correction du test, deux passages sur cible partagée avaient échoué au même endroit (275/1/48, code 101). Le premier journal complet reste dans [échec historique](raw/garde-n1/workspace-test-corrected.failed.log) ; l’autre reste dans l’historique Git. Les sorties workspace dédiées contiennent désormais le passage réussi. La [chronologie](raw/garde-n1/executions.txt) distingue les tentatives, dont un ancien succès sur une cible locale au worktree qui ne constituait pas la preuve 4 requise.

## Limites

Espace libre : environ **166 Gio avant**, **166,46 Gio après** cette validation. Neuf fichiers de preuves et un rapport, sans nouveau fichier ; cache et temporaires conservés sous target, aucun nettoyage. Non vérifié : auto-test de capture 0/23 sur la garde finale, compilations froides A/B/D réelles, release, recettes natives/matérielles, fenêtre Windows et absence d’écritures hors racines par confinement. Aucun format/build/Clippy distinct relancé dans cette validation. Aucun push.