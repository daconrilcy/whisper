Avancée : plan séquencé en sept tâches pour les 37 diagnostics des tests et supports.
Blocage : aucun changement Rust ni Cargo avant l’accord précis et une voie gardée admise pour la matrice complète.
Décision : commencer par les deux derniers réemprunts d’option, puis corriger les autres groupes dans l’ordre.

# Plan — prochaines corrections Clippy

**ID :** P-WHISPER-CLIPPY-SUIVANTES-20261113

**Design :** [D-WHISPER-CLIPPY-SUIVANTES-20261113](cadrage-corrections-clippy-suivantes-20261113.md), revue `CLEAN` au commit `4a5d61dd`; correction éditoriale sans changement de critères à `ab07eaed`.

**Base Rust :** `main@9f1b09c8b1977aa96afeccc080e7048c5fc32961`.

**État :** plan accepté pour le lot après accord utilisateur du 13 novembre 2026 ; implémentation Rust autorisée dans les sept chemins déclarés, sous réserve de T0 et des gates ci-dessous.

## Périmètre

- 37 diagnostics de tests/support dans sept fichiers, plus aucun fichier de production.
- Aucun manifeste, lockfile, protocole IPC, baseline, garde, seuil, fixture PE ou code système modifié.
- Aucun `allow`/`expect`, retrait d’assertion, affaiblissement de scénario, `native-tests`, `--ignored`, `--include-ignored`, build natif ou campagne GPU niveau 2.
- Les corps `ignore`/`native-tests`, l’exécution CUDA réelle et la validation dans la fenêtre Windows restent `NON EXÉCUTÉS` dans ce lot.
- Les décisions non démontrées pour un site individuel le diffèrent avec motif ; elles ne bloquent pas le reste si les fichiers et validations demeurent correctement attribués.

## Ancrages et chemins d’écriture autorisés

Les lignes sont des ancrages du relevé frais, pas des clés d’identité après extraction. Comparer par lint, crate, chemin normalisé, fonction/binding/opérande et rang de réemprunt lorsque nécessaire.

| Fichier | Sites de départ |
|---|---|
| `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs` | `unused_variables` 2205, 3376, 3377, 3614 ; `collapsible_if` 2131 ; `needless_borrow` 3628 ; appel `unreachable!` à 2298 |
| `crates/whisper-adapters/src/worker_ipc/tests/native_execution_helpers.rs` | `dead_code` sur `NativeGpuResumeRun.app` (champ à 67 ; initialiseur à vérifier dans `native_execution.rs`) |
| `crates/whisper-adapters/src/worker_ipc/tests/unified_capture_helpers.rs` | `unused_mut` 87 ; `unused_variables` 211, 276, 374, 448, 453, 455, 456 |
| `crates/whisper-adapters/tests/parakeet_unified_native.rs` | 2 `needless_option_as_deref` (444, 493) |
| `crates/whisper-adapters/tests/support/native_import_workflow.rs` | 7 `needless_option_as_deref` (46, 59, 85, 177, 219, 286, 376) |
| `crates/whisper-adapters/tests/reprocessing_cleanup.rs` | 6 `unused_variables` (907, 970, 971, 1031, 1091, 1092) |
| `crates/whisper-adapters/tests/support/reprocessing_cleanup_helpers.rs` | 5 `unused_variables`/paramètres (30, 79, 369, 468, 474) ; `dead_code` sur `ReplacementScenario.root` (champ à 4) |

Les modules de montage/chronométrage, les appelants natifs, les fichiers de définition et `ipc/mod.rs` ne sont que des entrées de lecture pour vérifier le contexte, les appels, les temps de vie et `Drop` ; aucun autre chemin Rust d’écriture n’est autorisé.

## Tâches

| Tâche | Travail et dépendances | Critère de sortie |
|---|---|---|
| **T0 — Préflight, admission et coordination** | Dans un worktree dédié, lire ce plan, le cadrage, `AGENTS.md`, les règles de crate et la référence Clippy courante. Vérifier HEAD, les sept chemins, les worktrees/branches concernés, les modifications concurrentes pertinentes, le profil observé, l’espace libre (≥30 Gio), cache hors ligne, racines sans reparse point et la voie de garde pour le `LotRoot` exact. Le split TOP5 est déjà contenu dans la base `9f1b09c8`; ses anciens documents ne constituent pas une autorisation. Inspecter statiquement la matrice N1 et les manifestes/build scripts : `whisper-worker-cpu` dépend de `whisper-rs` et peut déclencher CMake via `whisper-rs-sys`; une cible fraîche ou un cache présent ne prouve pas que le build natif sera évité. Le modèle personnel persistant Parakeet doit exister, porter son nom attendu et avoir le SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`. Obtenir accord précis d’implémentation et de voie de validation avant édition Rust ou commande Cargo. | Périmètre identique au relevé, aucune collision active et voie couvrant Clippy + toute la N1. **Bloquant tant que Q-01/Q-02 ou l’accord manque.** La sélection CPU ne sert jamais de sonde. Sous la suspension native, si le N1/default ou la sélection `whisper-worker-cpu` peut compiler `whisper-rs-sys`, arrêter avant Cargo et demander l’arbitrage/autorisation précise ou une révision approuvée de la validation. Ne pas appeler `DryRun` si sa validation déclenche metadata. Ne pas créer/étendre la garde, réutiliser une autre racine, télécharger une dépendance ou poursuivre si le préflight échoue. |
| **T1 — Réemprunts finaux : cas pilote** | Dans `tests/parakeet_unified_native.rs`, examiner puis corriger les deux derniers `as_deref_mut` de `Option<&mut NativeImportTiming>`. Garder les réemprunts antérieurs nécessaires et l’ordre des points de mesure `Some`/`None`. Première modification, avant T2. | Deux diagnostics ciblés disparus ou reportés individuellement avec motif. Clippy JSON gardé sur le même profil ; cliquet PASS, aucun nouveau site. Les mesures, marqueurs et traces restent identiques. |
| **T2 — Support des imports chronométrés** | Après T1, dans `tests/support/native_import_workflow.rs`, traiter les sept derniers réemprunts signalés. N’ajuster les `mut` de signatures que si leur inutilité résulte directement de ces changements. | Sept sites attribués. Même branche, valeur, résultat `Some`/`None`, ordre et points de mesure. Aucun helper ou paramètre ajouté pour satisfaire Clippy seul. |
| **T3 — Variables et paramètres** | Traiter les 22 sites dans `native_execution.rs` (4), `unified_capture_helpers.rs` (7), `reprocessing_cleanup.rs` (6) et `support/reprocessing_cleanup_helpers.rs` (5). Distinguer variable pure, expression avec effet, emprunt et propriétaire gardé en vie. Dans `prepare_replacement_scenario`, vérifier les deux appels qui transmettent les paramètres inutilisés `latest_generation` et `source` avant de les retirer. | Chaque changement conserve évaluations, appels, durée de vie, assertions et identités. Ne pas retirer une valeur qui retarde un `Drop` ni une expression à effet. Diff limité aux fichiers prévus et attribué par site. |
| **T4 — Diagnostics isolés** | Après T3, traiter `collapsible_if`, `needless_borrow` et le site `unreachable_code` dans `native_execution.rs`, puis `unused_mut` dans `unified_capture_helpers.rs`. L’avertissement unreachable se compte au site d’appel `unreachable!` à 2298, pas au span primaire dans la bibliothèque Rust. | Court-circuit, valeurs empruntées, boucle divergente, retours/erreurs, délais et assertions identiques. Aucune branche ou oracle supprimé ; rapporter la compilation séparément de l’exécution du corps de test. |
| **T5 — Champs inutilisés** | Après T3, vérifier le champ `NativeGpuResumeRun.app` et son initialiseur dans `native_execution.rs` ; vérifier tous lecteurs/constructeurs de `ReplacementScenario.root`. | Garder le propriétaire runtime/app jusqu’aux dernières assertions de reprise et libération ; un nom `_app` peut expliciter cette possession sans changer type, ordre ni moment de destruction. Retirer uniquement le champ `root` si l’audit confirme qu’il ne porte aucun effet/temps de vie. Chaque site est fermé ou différé avec preuve. |
| **T6 — Validation, revue et clôture** | Contrôler le diff, les sept chemins, les noms/attributs de tests et les sites Clippy par identité. Exécuter format, Clippy et N1 uniquement via la voie autorisée établie en T0 ; confier le candidat au coordinateur de revue Rust. Sur chaque correction de l’implémenteur après revue, relancer les vérifications concernées ; après le dernier cycle, revalider la matrice complète sur l’arbre final avant clôture. | Tous les 37 sites sont supprimés ou différés individuellement ; aucun nouveau diagnostic structurel, cliquet PASS, baseline inchangée. Revue `CLEAN` ou réserves explicites, au plus deux cycles. Les résultats finaux correspondent au candidat corrigé, et les tests/ignorés/limites sont attribués ; aucun test natif annoncé exécuté. |

## Vérifications conditionnées à T0

Toutes les commandes ci-dessous sont des exigences futures du lot, pas une autorisation présente. Elles sont soumises à l’espace, aux variables isolées, aux caches, aux racines et aux arrêts de `AGENTS.md`. Le Clippy frais existant n’est réutilisable comme référence que si la base et le profil admis concordent. Une absence de cache ou un besoin de `fetch` arrête avant Cargo ; aucun téléchargement n’est prévu.

| Vérification | Profil requis | Preuve et limite |
|---|---|---|
| SelfTest et PE | SelfTest de la garde et `ci/check-worker-imports.ps1 -SelfTest` ; contrôler les harnais existants/produits par type et chemin exact avant tout test worker. | PASS préalables. Témoin réel PE : `NON EXÉCUTÉ` sans sélection explicite et SHA-256. Le SelfTest de garde effectue ses sondes metadata selon son implémentation actuelle. |
| Format | `fmt --check` via la garde. | Aucun formatage automatique. |
| Clippy pilote T1 et candidat complet T6 | Six membres : `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-parakeet-cpu` ; `--all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`. Profil hors ligne/verrouillé, cible unique admise, JSON capturé. | Deux passages initiaux au plus avant la première revue (T1, T6). Chaque correction issue de la revue doit ensuite être revalidée ; après le dernier cycle, la matrice complète doit réussir sur l’arbre final. Même profil et base de comparaison ; cliquet PASS et aucune clé nouvelle. Si cible neuve, la créer seulement en T1 avec `-FreshTarget`, puis utiliser l’existante sous garde ; sinon cible canonique autorisée. |
| N1 membres par défaut | `test --no-fail-fast` sous garde, dans un shell dédié, `-WithoutCargoTargetDir`, cible canonique vérifiée par la garde et `-PersonalParakeetModelPath <chemin-persistant-vérifié>` (nom attendu et SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`). | Code de sortie et décompte des tests/ignorés. Enfant sans `CARGO_TARGET_DIR`; `target_directory` effectif conforme. Aucun téléchargement du modèle. Si la matrice défaut risque un build natif suspendu, le test ne démarre pas avant décision Q-02. |
| N1 workers CPU | Pour `whisper-worker-cpu`, `whisper-worker-parakeet-cpu`, `whisper-worker-common` : `test --no-run -p <membre>`, contrôle PE exact, `test -p <membre> -- --list`, puis `test -p <membre>`, tous via garde et même cible. | Ordre/type de harnais : Cpu, ParakeetCpu, Cpu. Aucun glob PE. La première sélection peut compiler `whisper-rs-sys`; elle est interdite sous la suspension native sans décision Q-02 et une voie conforme. Les corps ignorés et les voies native-tests restent `NON EXÉCUTÉS`. |

Le lot n’ajoute ni test ni hook uniquement pour le lint. Toute correction dont l’équivalence ne peut être établie est laissée en place avec motif précis. Aucune restauration globale ou suppression de cible/cache n’est implicite ; la clôture et la rétention suivent les règles du dépôt.

## Dépendances et autorisations

### Complément accepté — admission de la voie gardée (13 novembre 2026)

L’utilisateur a autorisé l’implémentation et la voie N1 pouvant compiler `whisper-worker-cpu`/`whisper-rs-sys` malgré la suspension native. Pour Q-01, le coordinateur fixe le `LotRoot` exact à `target/parakeet-tests/clippy-corrections-suivantes-20261113` dans le worktree dédié `C:/wt/clippy-corrections-suivantes`.

Le profil de la garde est limité à ce `LotRoot` et aux commandes suivantes, qui sont déjà prévues dans la matrice de T6 : `fmt --check`, `test --no-fail-fast`, pour chacun des workers CPU/Parakeet CPU/common `test --no-run -p <membre>`, `test -p <membre> -- --list` et `test -p <membre>`, ainsi que le Clippy exact à six membres documenté en T6. Les auto-tests refusent tout profil différent, GPU, membre supplémentaire et racine sœur/enfant non enregistrée. Les sélections passent toujours par metadata protégé, la garde et le contrôle PE requis. Aucun changement de baseline, seuil, garde générale, règle de sécurité ou catégorie de commande Cargo n’est permis par ce complément.

Q-02 est résolue par l’autorisation explicite de l’utilisateur ; le risque CMake/`whisper-rs-sys` reste signalé, et les règles de préflight, arrêt sur échec, cache hors ligne, isolation et suspension de toute recette DLL demeurent applicables. Aucun téléchargement, `native-tests`, build DLL, test GPU niveau 2, validation Windows, commit, merge ou push n’est autorisé par cet accord.

La modification de `ci/guard-level1.ps1` et de son SelfTest est limitée à l’enregistrement et à la vérification de ces profils pour cette racine. Elle est un préalable de T0. Le code Rust reste limité aux sept fichiers du plan et à l’implémenteur délégué ; toute divergence de profil ou de préflight arrête le lot.

```text
T0 (admission gardée) → T1 → T2 → T3 → T4 → T5 → T6
```

`T3` précède `T4` et `T5` parce que ces tâches se recoupent dans `native_execution.rs`, `unified_capture_helpers.rs` et `reprocessing_cleanup_helpers.rs`. Chaque tâche conserve un diff vérifiable avant la suivante. Les réserves sur le `LotRoot` non admis et Q-01/Q-02 du texte initial sont résolues uniquement par le complément accepté ci-dessus. La sélection `whisper-worker-cpu` reste un vrai test N1 soumis à l’autorisation accordée, jamais une sonde. Si le profil exact est refusé, si l’entrée change, si le préflight échoue, ou si un nouveau diagnostic structurel apparaît, arrêter selon `AGENTS.md` sans élargir garde, baseline ou commande de campagne.

**Accord utilisateur du 13 novembre 2026 :** implémentation des sept tâches Rust et admission de la voie gardée précise définie au complément ci-dessus ; validation N1 susceptible de compiler `whisper-rs-sys` autorisée sous la garde et les protections existantes. Le push, le merge, les téléchargements, `native-tests`, les builds DLL et la validation Windows restent non autorisés.
