Avancée : garde livrée avec réserves ; attente bornée ajoutée au test de remplacement du worker, sans nouvelle exécution.
Blocage : preuve 4 sur cible partagée en échec (code 101), confirmé par l'unique reprise autorisée.
Décision : LIVRÉ AVEC RÉSERVES ; deux cycles de correction consommés, aucun push.

# GARDE-N1 — catégories de commandes Cargo

Branche `garde/n1-categories`, worktree `C:/dev/whisper/target/agent-worktrees/garde-n1-categories`, base `refacto/tests-autonomes` **939fbbbe**. Plan accepté versionné avant réalisation : **09520288** (historique de ce fichier). Le lot initial débloque les diagnostics des workers locaux, notamment R8, sans changement Rust, manifeste, lockfile, modèle ou garde historique. Le correctif de test ultérieur, autorisé séparément, est décrit en fin de rapport.

| Catégorie | Commandes | Conditions |
| --- | --- | --- |
| Lecture | `tree`, `metadata`, `fmt --check` | arguments explicitement autorisés ; environnement isolé |
| Compilation | `check`, `build`, `test --no-run`, `clippy` | workspace ou un/plusieurs membres `-p`, release permis |
| Tests | `test` | ni ignored/include-ignored, ni feature native-tests |

La garde valide les membres par metadata protégé. Cible par défaut : `C:/dev/whisper/target/parakeet-tests/cargo-shared/`. `-LotRoot` désigne une racine contrôlée dans le worktree ; `-FreshTarget <nom>` impose un répertoire neuf sous cette racine. `-UnsetEnv` accepte seulement CUDA_PATH, CUDA_PATH_V*, GGML_*, WHISPER_* et PARAKEET_*. Les chemins canoniques et liens, l’espace minimal, CARGO_HOME/TEMP/TMP/LOCALAPPDATA isolés, stdout/stderr, codes de sortie et offline/locked restent contrôlés ; aucune surveillance de fichiers. `-DryRun` ne lance pas la commande demandée, mais peut lire les membres via metadata protégé. Le mode Structure existant est conservé (route seulement vérifiée en DryRun).

Refus par défaut de tout argument inconnu ; refus explicites : run/install/uninstall/update/publish/clean/add/remove/fix/generate-lockfile/vendor et commandes externes non listées ; --fix, --allow-dirty, --all-features, native-tests (y compris variantes qualifiées), recette-hooks en release, --ignored, --include-ignored, --config, -Z, --target-dir et manifest-path hors worktree.

Exemple : `& ./ci/guard-level1.ps1 -FreshTarget A -DryRun -CargoArgs @('build','-p','whisper-worker-cpu')` dans PowerShell.

## Preuves 1 à 4

| Preuve | Résultat et référence |
| --- | --- |
| 1 — auto-tests | **PASS**, `-Mode SelfTest` : **20 positifs / 49 négatifs**, code 0 ; validation sans Cargo, cas de jonction, racines et variables. La capture 0/23 n'a pas été auto-testée sur ce candidat. [selftest](raw/garde-n1/selftest.txt), [corrections](raw/garde-n1/correction-cycle1.txt). |
| 2 — R8 | **PASS**, six commandes du diagnostic plus D/UnsetEnv, cibles neuves A2/B2/D, commandes demandées non exécutées. [Sorties](raw/garde-n1/r8-dry-runs.txt) et invocations dans le journal des corrections. Réalisé après cycle 1 ; cycle 2 rétablit les contrôles d’environnement. |
| 3 — tree réel | **PASS**, `tree --offline --locked -p whisper-worker-cpu -e features -i whisper-rs-sys`, code 0 sur cible partagée après cycle 1. [stdout](raw/garde-n1/tree.stdout.log), [stderr](raw/garde-n1/tree.stderr.log). |
| 4 — tests workspace | **FAIL**, `test --offline --locked --workspace`, code **101** sur cible partagée : premier groupe **275 réussis / 1 échec / 48 ignorés**. [Journal complet](raw/garde-n1/workspace-test-corrected.failed.log). Arrêt, puis unique reprise autorisée également en échec au même endroit (voir ci-dessous). |

Le test en échec est `worker_ipc::tests::native_executor_backend_changes_only_after_the_prior_instance_is_reaped` : « replacement CPU route must start after the GPU instance was reaped », `native_execution.rs:730`. Le lot ne corrige pas ce Rust. Après accord explicite utilisateur, une seule reprise sur la garde finale a reproduit cet échec : 275/1/48, code 101, sans nouvelle relance. Ses sorties sont conservées dans [stdout](raw/garde-n1/workspace-test.stdout.log) et [stderr](raw/garde-n1/workspace-test.stderr.log) ; le premier échec reste conservé séparément. Un premier passage avait réussi avec **686/0/148**, mais sur une cible locale au worktree : preuve historique conservée, insuffisante pour valider le passage final sur la cible partagée ([chronologie](raw/garde-n1/executions.txt)).

La référence initiale 687/0/147 a été remplacée par **686/0/148** avec accord utilisateur conditionnel confirmé : [bilan de la base](TESTS-AUTONOMIE-INVENTAIRE.md), lignes 1 et 21, et attribut ignore déjà présent dans `capture_encoder_baseline` au commit 939fbbbe. Aucun Rust modifié dans ce lot.

## Revue et limites

Implémentation déléguée ; revue indépendante du même reviewer. Cycle 1 : six constats corrigés (features, racine avant écriture, cible canonique, Structure, UnsetEnv, options par sous-commande). Cycle 2 : rétablissement du refus de variables Whisper/Parakeet résiduelles et du nettoyage RUSTFLAGS pour Clippy. Parse PowerShell et `git diff --check` réussis. Revue finale : R1–R7 fermés, aucun constat source ouvert ; verdict global BLOCKED faute de preuve 4 réussie. Le compte rendu a été relu ; une affirmation non étayée de capture 0/23 a été retirée (limite explicitée ci-dessus). Aucune troisième correction de code autorisée.

Espace libre : environ **171 Gio avant**, **166,6 Gio après**. Neuf fichiers de preuves sous `raw/garde-n1/`, un rapport ; cache et temporaires conservés sous target, aucun nettoyage. Non vérifié : compilations froides A/B/D réelles, release, exécution native/matérielle, fenêtre Windows et absence d’écritures hors racines par confinement. La preuve 4 finale n’est pas acquise.
## Correctif du test — plan accepté le 1er novembre 2026

Autorité : après diagnostic en lecture, l'utilisateur demande « fais les corrections necessaires », acceptant la correction minimale proposée. Cette demande étend le périmètre antérieur à un seul fichier Rust de test : `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs`, uniquement le test `native_executor_backend_changes_only_after_the_prior_instance_is_reaped`. La consigne précédente de ne pas relancer Cargo est conservée ; aucun nouveau test exécuté n'est revendiqué.

1. L'implémenteur remplace l'appel unique à `poll_unified_sessions().expect(...)` par une boucle bornée de cinq secondes, avec pause de deux millisecondes sur `None` et échec explicite à échéance. Le premier événement `Some` est conservé pour l'assertion existante ; aucune erreur n'est ignorée.
2. Préserver les assertions Ready/JobId/backend CPU, la vérification du nouvel exécuteur et le scénario NativeRouteBusy. Aucun code de production, garde, manifeste, lockfile ou autre test modifié.
3. Vérifier le diff statiquement, puis revue indépendante via le coordinateur Rust, au plus deux cycles sur ce correctif distinct. Aucun Cargo ni rustc ; principal met à jour ce bilan et commit local sans push. Ne pas supprimer les preuves historiques en échec.

Critères : attente de Ready compatible avec le démarrage asynchrone ; attente bornée ; erreurs et événements incorrects toujours refusés ; delta limité au test ciblé et au présent bilan. L'exécution du test corrigé et la preuve 4 restent non vérifiées.

Réalisation du correctif : remplacement effectué uniquement dans le test nommé ; boucle bornée à cinq secondes, pause de deux millisecondes sur absence d'événement. Toutes les assertions existantes sont conservées. Contrôle statique du diff réussi. Aucun Cargo, rustc, build, test ou Clippy lancé ; la réussite du test et de la preuve 4 reste à établir. Revue indépendante statique : CLEAN, aucun constat ouvert ; ceci ne valide pas une exécution du test.
