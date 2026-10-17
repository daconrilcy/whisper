Avancée : les cinq scénarios R8 passent sur la clé Test R6 actuelle, avec sorties confinées à la racine R8.
Blocage : la suite workspace donne 22 échecs de prérequis de campagne et de helper de test ; la revue indépendante rend BLOCKED.
Décision : l'amendement v2 propose l'ouverture du seul helper `parakeet_archive.rs::archive_root()` et deux variables de campagne pour le second cycle.

# Bilan intermédiaire — reprise du R8 historique

Base : branche `wip/l2b-candidate`, HEAD avant candidat `4fbef019`. Le candidat Rust modifie seulement `crates/whisper-adapters/tests/parakeet_import.rs` ; les dix fichiers R7 déjà modifiés restent non commités. Le worker produit lu est `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`, le modèle `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` et les cinq DLL correspondent aux SHA du préflight R8. Les commandes sont hors réseau, avec Cargo partagé et sorties sous `target/parakeet-tests/l2/l2b/r8/`.

| Scénario | Résultat | Preuve principale |
| --- | --- | --- |
| DLL incompatible | PASS | `QualificationMismatch` à la vérification directe ; `ModelNotQualified` dans le parcours produit avant pending ; chargeur natif échoue avant Ready ; aucun TXT/SRT. |
| Worker tué après demande | PASS | `WorkerExited`, aucun résultat. |
| Trame Result incomplète | PASS | `IpcDecode` avec champ manquant. |
| Mort via le parcours produit | PASS | pending et source conservés, aucun Complete, aucun repli Turbo. |
| Trame incomplète via le parcours produit | PASS | pending et source conservés, aucun Complete, aucun repli Turbo. |

La garde a passé le cas valide (exit 0) et refusé `CARGO_TARGET_DIR=C:\dev\whisper\target` (exit 1) avant Cargo. Les sorties sont conservées dans `target/parakeet-tests/l2/l2b/r8/logs/guard-ok.log` et `guard-ko.log`. Les journaux de chaque cas et des deux suites workspace sont dans ce même répertoire.

`cargo test --offline --locked --workspace --no-fail-fast` a été lancé deux fois, avec et sans le flag de compilation des injections : **343 PASS, 22 FAIL, 29 ignorés** à chaque fois. Les échecs sont : deux tests unitaires `worker_ipc`, trois `legacy_model_requests`, un `model_catalog`, six `model_preferences` (racine `WHISPER_L1B_RUNTIME_ROOT` absente) ; huit tests `parakeet_archive` (helper n'admet pas R8) et un cas enfant R4 (audio long non fourni). Aucun de ces échecs ne vient d'une assertion des cinq cas R8. La suite workspace avant ce candidat était verte sous la garde R7, mais cette preuve ne remplace pas le gate R8.

Le premier cycle de revue indépendante ne relève pas de défaut R8 attribuable et rend `BLOCKED` pour le gate. Aucun code hors du chemin initialement autorisé n'a été modifié ; aucun commit de code ni push n'a été fait. Le delta précis pour un second cycle figure dans `docs/cadrage/plan-parakeet-r8-reprise.md` v2.
