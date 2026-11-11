Avancée : plan proposé pour supprimer sept alertes `type_complexity` par des alias locaux dans les supports de test IPC.
Blocage : la clôture future exige la N1 complète ; toute commande nécessitant un build natif suspendu doit s’arrêter.
Décision : retenir un plan de cinq tâches ; le code demande un GO précis distinct.

# Plan de refactorisation — Types des supports de test IPC

**Identifiant :** P-WHISPER-CLIPPY-ALIASES-01
**Statut :** proposé, non autorisé à l’implémentation.
**Parent :** [D-WHISPER-CLIPPY-ALIASES-01](evo-clippy-aliases-tests.md), DESIGN READY, contenu revu CLEAN au commit `2f31c617` et statut versionné au commit `5d0ae9d2`.
**Autorité :** l’utilisateur a demandé le cadrage et le plan. Un GO précis couvrant le lot Rust, ses chemins et ses vérifications reste nécessaire avant le code.

## OBJECTIF ET PORTÉE

Clarifier les types des fixtures de service, capture et publication de `whisper-adapters` tout en retirant les sept diagnostics `type_complexity` confirmés. Le Clippy ciblé du 11 novembre a réussi avec `keys=200 sites=201` et un cliquet PASS. Il n’a pas exécuté les tests.

| Chemin Rust autorisé dans le futur lot | Éléments et changement prévu |
|---|---|
| `crates/whisper-adapters/src/worker_ipc/service/mod.rs` | Ajouter un alias de tuple test-only pour le retour à sept positions de `AsyncImportIo::for_test`, même `#[cfg(test)]` et `pub(super)`. Employer `AsyncImportIo` dans l’alias à la place de `Self`. |
| `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs` | Ajouter un alias local pour le type de file `controlled_capture_blocks`, en conservant `Option`, puis un alias commun aux retours identiques de `held_worker`, `held_final_a_worker` et `held_worker_with_mode`. |
| `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs` | Ajouter un alias du message audio/acquittement de `samples_rx` et réutiliser l’alias existant `PublicationGateRegistry` pour `gates`. |

Maximum : quatre alias nouveaux, un alias existant réutilisé. Les fichiers de tests sont inclus dans le même module par `worker_ipc/tests.rs` ; vérifier la portée des noms avant édition. Les destructurations et appelants dans les autres fichiers sont à lire, mais restent hors des chemins d’écriture.

**Exclus :** logique produit, représentation des tuples, seuils/`allow`, baseline ou garde, autres lints ou crates, worker Parakeet de production, manifests, `Cargo.lock`, dépendances, nouveaux tests, build natif, exécution GPU, validation Windows, merge et push. Aucun test ignoré ou conditionné par une feature ne sera rendu actif par ce lot.

## SÉQUENCE DES TÂCHES

| Tâche | Critères | Actions, sorties et dépendances |
|---|---|---|
| **T1 — Préflight et mesure avant code** | AC-01, AC-02 | **Prérequis :** plan accepté et GO précis. Créer/attacher un worktree dédié sous `C:/wt/` ; confirmer HEAD, état Git, chemins de crates/manifestes, baseline, toolchain, cache hors ligne, variables, absence de jonctions et espace libre ≥30 Gio. Lire directement les sept diagnostics Clippy JSON sur la base du lot. Exécuter le SelfTest N1 de la garde. Arrêter si source/configuration diverge ou si un build natif suspendu est requis. |
| **T2 — Introduire les alias** | REQ-01/02, UC-01, AC-03 | **Prérequis :** T1 conforme. Modifier seulement les trois chemins du tableau. Nommer le message de capture, la file contrôlée, le tuple des workers et le tuple `for_test`; réutiliser `PublicationGateRegistry`. Conserver types sous-jacents, positions, capacités de canaux, files, verrouillages, corps, appels, assertions, visibilité et `cfg(test)`. Ne convertir aucun tuple en structure. |
| **T3 — Vérifier individuellement les diagnostics** | UC-02, AC-01/02 | **Prérequis :** T2 et diff attribué. Lancer Clippy ciblé gardé sur `whisper-adapters --all-targets` avec les quatre lints structurels. Comparer la sortie JSON avant/après par code lint, chemin normalisé, champ ou fonction et contexte/span primaire ; inclure les déclarations d’alias afin de vérifier qu’aucun diagnostic n’a été déplacé dessus. Le cliquet doit passer, la baseline reste inchangée, et les sept sites doivent avoir chacun disparu. Un total attendu de 194 sites structurels est seulement un rapprochement si la configuration demeure identique, jamais le critère principal. |
| **T4 — Format et clôture N1** | AC-03 | **Prérequis :** T3 réussi. Exécuter `cargo fmt --check`, les tests concernés et toute la suite N1 prescrite ci-dessous via les gardes. Chaque sélection worker passe le contrôle PE de ses harnais avant inventaire/exécution. Les tests ignorés ou conditionnés par une feature absente restent explicitement `NON EXÉCUTÉS`. Si une commande sélectionne indirectement un build natif suspendu ou CUDA, arrêter et demander l’arbitrage avant reprise. |
| **T5 — Revue et clôture** | AC-01 à AC-03 | **Prérequis :** candidat, diff, rapprochement JSON et résultats T3/T4 disponibles. Confier la revue du code à `rust_review_coordinator`, avec `rust_reviewer` indépendant. Limite de deux cycles revue/correction. Rapporter tests exécutés, ignorés, diagnostics résiduels et limites ; archiver les petites preuves puis appliquer la rétention exacte de la cible dédiée. Aucun push. |

## COMMANDES PROPOSÉES

Toutes les commandes ci-dessous sont **NON EXÉCUTÉES sur un candidat**. Elles supposent le worktree dédié `C:/wt/clippy-aliases-tests` (à confirmer au préflight), PowerShell et la toolchain du dépôt `1.98.1`. Chaque Cargo passe par `ci/guard-level1.ps1`, jamais en direct. La garde impose `--offline --locked` où applicable et les racines isolées prescrites.

```powershell
$lot = 'target/parakeet-tests/clippy-aliases-tests'
$target = 'clippy-aliases-tests'
$clippy = @('clippy','-p','whisper-adapters','--all-targets','--message-format=json',
    '--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity',
    '-W','clippy::too_many_arguments','-W','clippy::type_complexity')

# T1 : SelfTest puis référence avant édition, cible neuve
& ./ci/guard-level1.ps1 -Mode SelfTest -LotRoot $lot -FreshTarget $target
& ./ci/guard-level1.ps1 -Mode Run -LotRoot $lot -FreshTarget $target -CargoArgs $clippy

# T3 : candidat, même sélection et cible
& ./ci/guard-level1.ps1 -Mode Run -LotRoot $lot -ExistingTarget $target -CargoArgs $clippy

# T4 : format et tests du membre modifié
& ./ci/guard-level1.ps1 -Mode Run -LotRoot $lot -ExistingTarget $target `
    -CargoArgs @('fmt','--check')
& ./ci/guard-level1.ps1 -Mode Run -LotRoot $lot -ExistingTarget $target `
    -CargoArgs @('test','-p','whisper-adapters')
```

La clôture d’un lot Rust exige aussi la N1 de `AGENTS.md` : `cargo test` des membres par défaut, puis `cargo test -p whisper-worker-cpu`, `cargo test -p whisper-worker-parakeet-cpu` et `cargo test -p whisper-worker-common`. Commencer par le SelfTest de `ci/check-worker-imports.ps1`. Pour chaque sélection worker, compiler les harnais avec `cargo test -p <membre> --no-run` gardé, contrôler individuellement chaque chemin exact produit, puis exécuter l’inventaire `-- --list` gardé et les tests gardés. Le contrôle PE utilise `Cpu` pour `whisper-worker-cpu` et `whisper-worker-common`, `ParakeetCpu` pour `whisper-worker-parakeet-cpu`. Le GPU est limité à `cargo test -p whisper-worker-gpu -- --list` via la garde, pour comparer les identités ; aucun corps de test GPU n’est exécuté.

Dans un shell dédié neuf, vérifier l’absence de `CARGO_TARGET_DIR` puis effectuer la commande N1 spéciale :

```powershell
& ./ci/guard-level1.ps1 -Mode Run `
    -LotRoot 'target/parakeet-tests/clippy-aliases-tests' `
    -ExistingTarget 'clippy-aliases-tests' -WithoutCargoTargetDir `
    -CargoArgs @('test','--no-fail-fast')
```

Attendus : succès des sélections, `CARGO_TARGET_DIR_child=False` pour cette commande spéciale et `target_directory` contrôlé par la garde. Confirmer avant chaque commande le cache hors ligne, les variables, l’espace libre, les racines et l’absence de concurrence pertinente. La commande Clippy workspace n’est pas incluse : elle a déclenché CMake CUDA et échoué ; aucun build CUDA ou native n’est autorisé par ce plan. La campagne N1 doit s’arrêter si son préflight révèle qu’une compilation relève de la suspension native, sans contourner les gardes.

## RISQUES, ARRÊTS ET RETOUR

- **R-01 — Positions des tuples :** les tuples contiennent des membres de types proches ; préserver l’ordre et les destructurations, même si le compilateur accepte une permutation.
- **R-02 — Déplacement d’alerte :** le cliquet peut passer sans prouver le retrait individuel de chaque diagnostic ; le rapprochement JSON par site inclut donc aussi les alias.
- **R-03 — Portée commune des tests inclus :** éviter les collisions de noms ; aucun alias ne devient API de production. Si une alerte persiste sur l’alias ou exige une conversion de représentation, arrêter le site et réviser le design.
- **R-04 — Validation native :** un échec de build/test ou une nécessité de compilation CUDA/native arrête la campagne ; consigner le chemin, conserver le candidat et les journaux, puis demander arbitrage avant reprise.
- **Retour arrière :** le delta Rust est borné aux trois chemins. En cas d’échec, préserver les modifications concurrentes et annuler uniquement le delta attribuable à ce lot ; ne pas restaurer globalement les fichiers.
- **Rétention :** archiver les petites preuves hors de `target/` dans l’emplacement persistant autorisé, préserver l’arborescence et vérifier nombre/SHA-256. Supprimer uniquement la cible neuve exacte sous `target/parakeet-tests/clippy-aliases-tests` après vérification des chemins, jonctions et processus. Préserver `target/cargo-home-shared` et les caches persistants.

## RAPPORT

Le plan est borné à sept diagnostics test-only, trois fichiers et cinq tâches. Il relie le succès du cliquet à une comparaison individuelle des diagnostics, puis à la N1 exigée pour tout lot Rust. Le plan ne donne pas de GO code : la décision d’implémentation et tout arbitrage déclenché par un build natif suspendu restent à obtenir séparément.
